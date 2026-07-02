//! Golden parity tests: the Rust renderer's output must match, byte for
//! byte, the output of the Go implementation on the same fixtures.
//!
//! The files under `testdata/golden/` were generated mechanically from
//! `apstndb/spannerplan@main` (see `DESIGN.md` §9); regenerate them with the
//! Go harness rather than editing by hand.

use spannerplan::core::model::PlanNode;
use spannerplan::core::reference::{
    render_tree_table, render_tree_table_with_config, Format, PrintSection, RenderConfig,
    RenderMode,
};
use spannerplan::extract::extract_plan_nodes;

fn testdata(rel: &str) -> String {
    format!("{}/../../testdata/{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn load_fixture(name: &str) -> Vec<PlanNode> {
    let path = match name {
        "dca" => testdata("reference/dca.yaml"),
        "dcaplan" => testdata("reference/distributed_cross_apply.yaml"),
        _ => panic!("unknown fixture {name}"),
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    extract_plan_nodes(&bytes).expect("extract plan nodes")
}

fn golden(name: &str) -> String {
    let path = testdata(&format!("golden/{name}.txt"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn assert_matches_golden(got: &str, golden_name: &str) {
    let want = golden(golden_name);
    assert_eq!(
        got, want,
        "output does not match Go golden {golden_name}.txt"
    );
}

#[test]
fn mode_format_matrix_matches_go() {
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
        let nodes = load_fixture(fixture);
        for (mode, mode_name) in modes {
            for (format, format_name) in formats {
                let got = render_tree_table(&nodes, mode, format, 0).unwrap();
                assert_matches_golden(&got, &format!("{fixture}_{mode_name}_{format_name}"));
            }
        }
    }
}

#[test]
fn wrap_width_and_hanging_indent_match_go() {
    for fixture in ["dca", "dcaplan"] {
        let nodes = load_fixture(fixture);
        for width in [50, 80] {
            let got = render_tree_table(&nodes, RenderMode::Plan, Format::Current, width).unwrap();
            assert_matches_golden(&got, &format!("{fixture}_plan_current_wrap{width}"));

            let got = render_tree_table_with_config(
                &nodes,
                RenderMode::Plan,
                Format::Current,
                &RenderConfig {
                    wrap_width: width,
                    hanging_indent: true,
                    ..RenderConfig::default()
                },
            )
            .unwrap();
            assert_matches_golden(&got, &format!("{fixture}_plan_current_wrap{width}_hanging"));
        }
    }
}

#[test]
fn print_sections_match_go() {
    struct Case {
        suffix: &'static str,
        sections: Vec<PrintSection>,
        show_scalar_vars: bool,
        resolve_scalar_vars_recursive: bool,
    }
    let cases = [
        Case {
            suffix: "enhanced",
            sections: vec![
                PrintSection::Predicates,
                PrintSection::Ordering,
                PrintSection::Aggregate,
            ],
            show_scalar_vars: false,
            resolve_scalar_vars_recursive: false,
        },
        Case {
            suffix: "full",
            sections: vec![PrintSection::Full],
            show_scalar_vars: false,
            resolve_scalar_vars_recursive: false,
        },
        Case {
            suffix: "typed",
            sections: vec![PrintSection::Typed],
            show_scalar_vars: false,
            resolve_scalar_vars_recursive: false,
        },
        Case {
            suffix: "enhanced_showvars_resolverec",
            sections: vec![
                PrintSection::Predicates,
                PrintSection::Ordering,
                PrintSection::Aggregate,
            ],
            show_scalar_vars: true,
            resolve_scalar_vars_recursive: true,
        },
    ];

    for fixture in ["dca", "dcaplan"] {
        let nodes = load_fixture(fixture);
        for case in &cases {
            let got = render_tree_table_with_config(
                &nodes,
                RenderMode::Plan,
                Format::Current,
                &RenderConfig {
                    print_sections: Some(case.sections.clone()),
                    show_scalar_vars: case.show_scalar_vars,
                    resolve_scalar_vars_recursive: case.resolve_scalar_vars_recursive,
                    ..RenderConfig::default()
                },
            )
            .unwrap();
            assert_matches_golden(
                &got,
                &format!("{fixture}_plan_current_print_{}", case.suffix),
            );
        }
    }
}
