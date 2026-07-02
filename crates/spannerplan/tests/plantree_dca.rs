//! Integration tests over the real dca.yaml fixture, transcribed from the
//! fixture-based tests in Go `plantree/plantree_test.go` (the synthetic-plan
//! tests live as unit tests in `spannerplan-core`).

use spannerplan::core::plantree::{process_plan, ProcessPlanOptions, RowWithPredicates};
use spannerplan::core::queryplan::{
    ExecutionMethodFormat, KnownFlagFormat, NodeTitleOptions, QueryPlan, TargetMetadataFormat,
};
use spannerplan::extract::extract_plan_nodes;

fn decode_dca_plan() -> QueryPlan {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../testdata/reference/dca.yaml"
    ))
    .expect("read dca.yaml");
    let nodes = extract_plan_nodes(&bytes).expect("extract plan nodes");
    QueryPlan::new(nodes).expect("build query plan")
}

fn current_options() -> ProcessPlanOptions {
    ProcessPlanOptions::default().with_query_plan_options(
        NodeTitleOptions::default()
            .with_target_metadata_format(TargetMetadataFormat::On)
            .with_execution_method_format(ExecutionMethodFormat::Angle)
            .with_known_flag_format(KnownFlagFormat::Label),
    )
}

fn row_by_id(rows: &[RowWithPredicates], id: i32) -> &RowWithPredicates {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("row with ID {id} not found"))
}

#[test]
fn current_formatting() {
    let rows = process_plan(&decode_dca_plan(), &current_options()).unwrap();

    struct Want {
        id: i32,
        tree_part: &'static str,
        node_text: &'static str,
        predicates: &'static [&'static str],
    }
    let tests = [
        Want {
            id: 0,
            tree_part: "",
            node_text: "Distributed Union on AlbumsByAlbumTitle <Row>",
            predicates: &[
                "Split Range: (STARTS_WITH($AlbumTitle, 'T') AND ($AlbumTitle LIKE 'T%e'))",
            ],
        },
        Want {
            id: 1,
            tree_part: "+- ",
            node_text: "Distributed Cross Apply <Row>",
            predicates: &["Split Range: (($SingerId' = $SingerId) AND ($AlbumId' = $AlbumId))"],
        },
        Want {
            id: 2,
            tree_part: "   +- ",
            node_text: "[Input] Create Batch <Row>",
            predicates: &[],
        },
        Want {
            id: 3,
            tree_part: "   |  +- ",
            node_text: "Local Distributed Union <Row>",
            predicates: &[],
        },
    ];

    for want in tests {
        let got = row_by_id(&rows, want.id);
        assert_eq!(got.tree_part, want.tree_part, "row {} tree_part", want.id);
        assert_eq!(got.node_text, want.node_text, "row {} node_text", want.id);
        assert_eq!(
            got.predicates, want.predicates,
            "row {} predicates",
            want.id
        );
    }
}

#[test]
fn wrap_width_preserves_tree_and_node_parts() {
    let opts = current_options().with_wrap_width(40);
    let rows = process_plan(&decode_dca_plan(), &opts).unwrap();

    let tests = [
        (0, "\n", "Distributed Union on AlbumsByAlbumTitle\n<Row>"),
        (
            5,
            "   |        +- \n   |           ",
            "Filter Scan <Row> (seekab\nle_key_size: 1)",
        ),
        (
            24,
            "         +- \n         |  ",
            "[Input] KeyRangeAccumulator\n<Row>",
        ),
    ];

    for (id, tree_part, node_text) in tests {
        let got = row_by_id(&rows, id);
        assert_eq!(got.tree_part, tree_part, "row {id} tree_part");
        assert_eq!(got.node_text, node_text, "row {id} node_text");
        if id != 0 {
            assert_ne!(
                got.text(),
                got.node_text,
                "row {id} Text() should include tree prefix"
            );
        }
    }
}

#[test]
fn tiny_wrap_width_does_not_panic() {
    let rows = process_plan(&decode_dca_plan(), &current_options().with_wrap_width(1)).unwrap();
    assert!(!rows.is_empty());
}

#[test]
fn wrap_width_zero_disables_wrapping() {
    let plan = decode_dca_plan();
    let base = process_plan(&plan, &current_options()).unwrap();
    let with_zero = process_plan(&plan, &current_options().with_wrap_width(0)).unwrap();
    assert_eq!(base, with_zero);
}

#[test]
fn compact_formatting() {
    let opts = current_options().enable_compact();
    let rows = process_plan(&decode_dca_plan(), &opts).unwrap();

    for (id, want) in [(0, ""), (1, "+"), (2, " +"), (3, " |+")] {
        assert_eq!(row_by_id(&rows, id).tree_part, want, "row {id} tree_part");
    }
}

#[test]
fn stats_extracted_from_real_profile() {
    // dca.yaml is a profile: the root row has real execution stats
    // (12.25 msecs latency, 386 rows — see the Phase 8 golden).
    let rows = process_plan(&decode_dca_plan(), &current_options()).unwrap();
    let root = row_by_id(&rows, 0);
    assert_eq!(root.execution_stats.rows.total, "386");
    assert_eq!(root.execution_stats.latency.to_string(), "12.25 msecs");
    assert_eq!(root.execution_stats.execution_summary.num_executions, "1");
}
