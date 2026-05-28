#[path = "json_payloads/captures.rs"]
mod captures;
#[path = "json_payloads/fragments.rs"]
mod fragments;
#[path = "json_payloads/render_run.rs"]
mod render_run;
#[path = "json_payloads/runners.rs"]
mod runners;
#[path = "json_payloads/sequences.rs"]
mod sequences;
#[path = "json_payloads/store_payloads.rs"]
mod store_payloads;

use super::*;

#[test]
fn json_success_payload_shapes_are_public_contract() {
    let store = TestStore::new("json-contract");

    store_payloads::assert_store_and_discovery_payloads(&store);
    fragments::assert_fragment_payloads(&store);
    sequences::assert_sequence_payloads(&store);
    render_run::assert_render_payload(&store);
    let _runner_sink = runners::assert_runner_payloads(&store);
    render_run::assert_run_payloads(&store);
    render_run::assert_saved_render_payload(&store);
    store_payloads::assert_history_payloads(&store);
    captures::assert_capture_payloads(&store);
    sequences::assert_sequence_remove_payload(&store);
    fragments::assert_fragment_remove_payload(&store);

    assert_git_clean(store.path());
}
