use std::fs;
use std::path::Path;

use spannerplan::core::plantree::structural_signature;
use spannerplan::core::queryplan::QueryPlan;
use spannerplan::extract::extract_plan_nodes;

#[test]
fn dca_structural_signature_matches_the_copied_go_golden() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let nodes = extract_plan_nodes(
        &fs::read(root.join("testdata/reference/dca.yaml")).expect("read DCA fixture"),
    )
    .expect("extract DCA plan nodes");
    let plan = QueryPlan::new(nodes).expect("validate DCA query plan");

    assert_eq!(
        structural_signature(&plan).expect("sign DCA plan"),
        fs::read_to_string(root.join("testdata/golden/dca.signature.txt"))
            .expect("read copied Go signature golden")
    );
}
