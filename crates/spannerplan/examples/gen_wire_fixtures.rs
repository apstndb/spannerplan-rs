//! Dev helper: write protobuf wire fixtures for JS wire parity tests.
//!
//! ```bash
//! cargo run -p spannerplan --example gen_wire_fixtures
//! ```

use std::fs;
use std::path::PathBuf;

use prost::Message;
use spannerplan::core::wire;
use spannerplan::extract::extract_plan_nodes;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../..");
    let out_dir = root.join("testdata/wire");
    fs::create_dir_all(&out_dir).expect("create testdata/wire");

    for (name, rel) in [
        ("dca", "reference/dca.yaml"),
        ("dcaplan", "reference/distributed_cross_apply.yaml"),
    ] {
        let path = root.join("testdata").join(rel);
        let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let nodes = extract_plan_nodes(&bytes).expect("extract plan nodes");
        let plan = wire::encode_query_plan_for_test(&nodes);
        let wire = plan.encode_to_vec();
        let out = out_dir.join(format!("{name}_query_plan.bin"));
        let len = wire.len();
        fs::write(&out, wire).expect("write wire fixture");
        eprintln!("wrote {} ({len} bytes)", out.display());
    }
}
