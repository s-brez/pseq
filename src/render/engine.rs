use std::collections::BTreeMap;

use crate::error::AppError;
use crate::turn_settings;

use super::INCLUDE_PREFIX;
use super::fragments::resolve_render_fragment;
use super::model::{FragmentIncludeFrame, RenderFragment, RenderSequence};
use super::types::*;
use super::variables::is_valid_variable_name;

pub(crate) fn render_turns_with_variables(
    store_path: &std::path::Path,
    sequence_reference: &str,
    variables: &BTreeMap<String, String>,
) -> Result<RenderedSequenceTurns, AppError> {
    crate::store::require_valid_store(store_path)?;
    let sequence = super::load::load_current_sequence(store_path, sequence_reference)?;
    render_sequence_turns(&sequence, variables)
}

pub(crate) fn render_sequence_turns(
    sequence: &RenderSequence,
    variables: &BTreeMap<String, String>,
) -> Result<RenderedSequenceTurns, AppError> {
    let turns = sequence
        .fragments
        .iter()
        .enumerate()
        .map(|(index, fragment)| {
            Ok(RenderedTurn {
                index: index + 1,
                fragment: rendered_turn_fragment(fragment),
                text: render_fragment_body(fragment, &sequence.catalog, variables)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(RenderedSequenceTurns {
        id: sequence.id.clone(),
        name: sequence.name.clone(),
        path: sequence.path.clone(),
        turns,
    })
}

pub(crate) fn render_sequence_runtime_turns(
    sequence: &RenderSequence,
    variables: &BTreeMap<String, String>,
) -> Result<RenderedSequenceRuntimeTurns, AppError> {
    let turns = sequence
        .fragments
        .iter()
        .enumerate()
        .map(|(index, fragment)| {
            let rendered_fragment = rendered_turn_fragment(fragment);
            let settings = turn_settings::fragment_turn_settings(
                fragment.pseq_metadata.as_ref(),
                fragment.dotted_reasoning_effort.as_ref(),
                &rendered_fragment,
            )?;
            Ok(RenderedRuntimeTurn {
                index: index + 1,
                fragment: rendered_fragment,
                settings,
                text: render_fragment_body(fragment, &sequence.catalog, variables)?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(RenderedSequenceRuntimeTurns {
        id: sequence.id.clone(),
        name: sequence.name.clone(),
        path: sequence.path.clone(),
        turns,
    })
}

pub(super) fn render_text(
    fragments: &[RenderFragment],
    catalog: &[RenderFragment],
    variables: &BTreeMap<String, String>,
    annotate: bool,
) -> Result<String, AppError> {
    let mut text = String::new();
    if annotate {
        for (index, fragment) in fragments.iter().enumerate() {
            append_annotated_fragment(&mut text, index + 1, fragment, catalog, variables)?;
        }
    } else {
        for fragment in fragments {
            text.push_str(&render_fragment_body(fragment, catalog, variables)?);
        }
    }

    Ok(text)
}

fn append_annotated_fragment(
    text: &mut String,
    position: usize,
    fragment: &RenderFragment,
    catalog: &[RenderFragment],
    variables: &BTreeMap<String, String>,
) -> Result<(), AppError> {
    let name = annotation_value(&fragment.name);
    let path = annotation_value(&fragment.path);
    text.push_str(&format!(
        "<!-- pseq fragment {position} begin: {name} ({}) {path} -->\n",
        fragment.id
    ));
    let rendered_body = render_fragment_body(fragment, catalog, variables)?;
    text.push_str(&rendered_body);
    if !rendered_body.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("<!-- pseq fragment {position} end -->\n"));
    Ok(())
}

pub(super) fn render_fragment_body(
    fragment: &RenderFragment,
    catalog: &[RenderFragment],
    variables: &BTreeMap<String, String>,
) -> Result<String, AppError> {
    let mut stack = vec![fragment_include_frame(fragment)];
    expand_placeholders(&fragment.body, catalog, variables, &mut stack)
}

fn expand_placeholders(
    text: &str,
    catalog: &[RenderFragment],
    variables: &BTreeMap<String, String>,
    include_stack: &mut Vec<FragmentIncludeFrame>,
) -> Result<String, AppError> {
    let mut rendered = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find("{{") {
        rendered.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            return Err(AppError::InvalidVariablePlaceholder {
                placeholder: rest[start..].to_owned(),
            });
        };

        let placeholder = &after_start[..end];
        if let Some(reference) = placeholder.strip_prefix(INCLUDE_PREFIX) {
            if reference.trim().is_empty() {
                return Err(AppError::InvalidFragmentInclude {
                    placeholder: format!("{{{{{placeholder}}}}}"),
                });
            }

            let fragment = resolve_render_fragment(catalog, reference)?;
            if let Some(index) = include_stack
                .iter()
                .position(|frame| frame.id == fragment.id)
            {
                let mut chain = include_stack[index..]
                    .iter()
                    .map(|frame| frame.label.clone())
                    .collect::<Vec<_>>();
                chain.push(fragment_label(fragment));
                return Err(AppError::FragmentIncludeCycle {
                    reference: reference.to_owned(),
                    chain: chain.join(" -> "),
                });
            }

            include_stack.push(fragment_include_frame(fragment));
            let expanded = expand_placeholders(&fragment.body, catalog, variables, include_stack)?;
            include_stack.pop();
            rendered.push_str(&expanded);
            rest = &after_start[end + 2..];
            continue;
        }

        let name = placeholder;
        if !is_valid_variable_name(name) {
            return Err(AppError::InvalidVariablePlaceholder {
                placeholder: format!("{{{{{name}}}}}"),
            });
        }

        let value = variables
            .get(name)
            .ok_or_else(|| AppError::MissingVariable {
                name: name.to_owned(),
            })?;
        rendered.push_str(value);
        rest = &after_start[end + 2..];
    }

    rendered.push_str(rest);
    Ok(rendered)
}

fn fragment_include_frame(fragment: &RenderFragment) -> FragmentIncludeFrame {
    FragmentIncludeFrame {
        id: fragment.id.clone(),
        label: fragment_label(fragment),
    }
}

fn rendered_turn_fragment(fragment: &RenderFragment) -> RenderedTurnFragment {
    RenderedTurnFragment {
        id: fragment.id.clone(),
        name: fragment.name.clone(),
        path: fragment.path.clone(),
    }
}

fn fragment_label(fragment: &RenderFragment) -> String {
    format!("{} ({})", fragment.name, fragment.path)
}

fn annotation_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ").replace("--", "- -")
}
