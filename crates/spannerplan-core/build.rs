fn main() {
    println!("cargo:rerun-if-changed=../../proto");

    if std::env::var("CARGO_FEATURE_WIRE").is_err() {
        return;
    }

    let proto_root = "../../proto";
    let mut fds = protox::compile(
        [
            "google/spanner/v1/query_plan.proto",
            "google/spanner/v1/result_set.proto",
        ],
        [proto_root],
    )
    .expect("compile vendored .proto files");

    // Parsed for imports only; runtime types come from prost-types.
    fds.file.retain(|f| {
        f.name
            .as_deref()
            .is_none_or(|name| !name.starts_with("google/protobuf/"))
    });

    prost_build::Config::new()
        .btree_map(["."])
        .compile_fds(fds)
        .expect("generate prost types from FileDescriptorSet");
}
