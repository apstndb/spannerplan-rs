//! Wire-vs-JSON parity: the wire decode path must produce the same model and
//! rendered output as the JSON/YAML path on golden fixtures.

use spannerplan::core::model::PlanNode;
use spannerplan::core::reference::{render_tree_table, Format, RenderMode};
use spannerplan::core::wire;
use spannerplan::extract::extract_plan_nodes;

fn testdata(rel: &str) -> String {
    format!("{}/../../testdata/{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn load_json_nodes(fixture: &str) -> Vec<PlanNode> {
    let path = match fixture {
        "dca" => testdata("reference/dca.yaml"),
        "dcaplan" => testdata("reference/distributed_cross_apply.yaml"),
        _ => panic!("unknown fixture {fixture}"),
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    extract_plan_nodes(&bytes).expect("extract plan nodes")
}

fn encode_query_plan(nodes: &[PlanNode]) -> Vec<u8> {
    use prost::Message;

    let plan = spannerplan::core::wire::encode_query_plan_for_test(nodes);
    plan.encode_to_vec()
}

#[test]
fn wire_decode_model_matches_json_for_fixtures() {
    for fixture in ["dca", "dcaplan"] {
        let json_nodes = load_json_nodes(fixture);
        let wire_bytes = encode_query_plan(&json_nodes);
        let wire_nodes = wire::decode_plan_nodes(&wire_bytes).unwrap_or_else(|e| {
            panic!("decode wire for fixture {fixture}: {e}");
        });
        assert_eq!(
            wire_nodes, json_nodes,
            "wire model mismatch for fixture {fixture}"
        );
    }
}

#[test]
fn wire_decode_wrapped_shapes_match_bare_query_plan() {
    use prost::Message;

    for fixture in ["dca", "dcaplan"] {
        let json_nodes = load_json_nodes(fixture);
        let bare = encode_query_plan(&json_nodes);
        let stats = wire::encode_result_set_stats_for_test(&json_nodes).encode_to_vec();
        let result_set = wire::encode_result_set_for_test(&json_nodes).encode_to_vec();

        let bare_nodes = wire::decode_plan_nodes(&bare).unwrap();
        assert_eq!(wire::decode_plan_nodes(&stats).unwrap(), bare_nodes);
        assert_eq!(wire::decode_plan_nodes(&result_set).unwrap(), bare_nodes);
    }
}

#[test]
fn wire_render_output_matches_json_for_golden_matrix() {
    let modes = [
        (RenderMode::Auto, "auto"),
        (RenderMode::Plan, "plan"),
        (RenderMode::Profile, "profile"),
    ];
    let formats = [
        (Format::Traditional, "traditional"),
        (Format::Current, "current"),
        (Format::Compact, "compact"),
    ];

    for fixture in ["dca", "dcaplan"] {
        let json_nodes = load_json_nodes(fixture);
        let wire_bytes = encode_query_plan(&json_nodes);
        let wire_nodes = wire::decode_plan_nodes(&wire_bytes).unwrap();

        for (mode, mode_name) in modes {
            for (format, format_name) in formats {
                let from_json = render_tree_table(&json_nodes, mode, format, 0).unwrap();
                let from_wire = render_tree_table(&wire_nodes, mode, format, 0).unwrap();
                assert_eq!(
                    from_wire, from_json,
                    "wire render mismatch for {fixture}_{mode_name}_{format_name}"
                );
            }
        }
    }
}
