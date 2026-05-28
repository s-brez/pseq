use super::*;

#[test]
fn render_fails_closed_for_ambiguous_sequence_reference() {
    let store = TestStore::initialized("render-ambiguity");
    create_sequence(&store, "Repeated");
    create_sequence(&store, "Repeated");

    assert_render_json_error(&store, "Repeated", "sequence_reference_ambiguous");
}
