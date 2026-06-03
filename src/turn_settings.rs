use crate::error::AppError;
use crate::render::RenderedTurnFragment;
use crate::yaml::Value;

pub(crate) const REASONING_EFFORT_KEY: &str = "pseq.run.reasoning_effort";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TurnRuntimeSettings {
    pub(crate) reasoning_effort: Option<ReasoningEffort>,
}

impl TurnRuntimeSettings {
    pub(crate) fn is_empty(self) -> bool {
        self.reasoning_effort.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl ReasoningEffort {
    pub(crate) const VALUES: &'static [&'static str] =
        &["minimal", "low", "medium", "high", "xhigh", "max"];

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "minimal" => Some(Self::Minimal),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "xhigh" => Some(Self::XHigh),
            "max" => Some(Self::Max),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }
}

pub(crate) fn fragment_turn_settings(
    pseq_metadata: Option<&Value>,
    dotted_reasoning_effort: Option<&Value>,
    fragment: &RenderedTurnFragment,
) -> Result<TurnRuntimeSettings, AppError> {
    if dotted_reasoning_effort.is_some() {
        return Err(invalid_setting(
            fragment,
            format!(
                "literal frontmatter key {REASONING_EFFORT_KEY:?} is not supported; use nested keys pseq -> run -> reasoning_effort"
            ),
        ));
    }

    let Some(pseq_metadata) = pseq_metadata else {
        return Ok(TurnRuntimeSettings::default());
    };
    let Some(pseq_mapping) = pseq_metadata.as_mapping() else {
        return Ok(TurnRuntimeSettings::default());
    };
    let Some(run_metadata) = pseq_mapping.get("run") else {
        return Ok(TurnRuntimeSettings::default());
    };
    let Some(run_mapping) = run_metadata.as_mapping() else {
        return Ok(TurnRuntimeSettings::default());
    };
    let Some(reasoning_effort) = run_mapping.get("reasoning_effort") else {
        return Ok(TurnRuntimeSettings::default());
    };

    let effort = reasoning_effort.as_str().ok_or_else(|| {
        invalid_setting(
            fragment,
            format!("{REASONING_EFFORT_KEY} must be a string value"),
        )
    })?;
    let effort = ReasoningEffort::parse(effort).ok_or_else(|| {
        invalid_setting(
            fragment,
            format!(
                "{REASONING_EFFORT_KEY} has unsupported value {effort:?}; expected one of {}",
                ReasoningEffort::VALUES.join(", ")
            ),
        )
    })?;

    Ok(TurnRuntimeSettings {
        reasoning_effort: Some(effort),
    })
}

pub(crate) fn fragment_setting_label(fragment: &RenderedTurnFragment) -> String {
    format!(
        "fragment {:?} ({})",
        fragment.name,
        fragment.path.replace('\\', "/")
    )
}

fn invalid_setting(fragment: &RenderedTurnFragment, message: String) -> AppError {
    AppError::InvalidRunInvocation {
        message: format!("{}: {message}", fragment_setting_label(fragment)),
    }
}
