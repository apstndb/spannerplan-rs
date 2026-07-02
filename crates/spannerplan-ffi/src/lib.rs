//! C ABI for spannerplan-rs. See `DESIGN.md` §8.1.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use spannerplan::core::reference::{
    parse_format, parse_render_mode, render_tree_table_with_config, RenderConfig,
};
use spannerplan::core::wire;
use spannerplan::extract::extract_plan_nodes;

fn render_internal(
    plan_nodes: Vec<spannerplan::core::model::PlanNode>,
    mode: &str,
    format: &str,
    config_json: *const c_char,
) -> Result<String, String> {
    let mode = parse_render_mode(mode).map_err(|e| e.to_string())?;
    let format = parse_format(format).map_err(|e| e.to_string())?;
    let config = parse_config_json(config_json)?;
    render_tree_table_with_config(&plan_nodes, mode, format, &config).map_err(|e| e.to_string())
}

fn parse_config_json(config_json: *const c_char) -> Result<RenderConfig, String> {
    if config_json.is_null() {
        return Ok(RenderConfig::default());
    }
    let raw = unsafe { CStr::from_ptr(config_json) }
        .to_str()
        .map_err(|e| format!("config_json is not valid UTF-8: {e}"))?;
    if raw.is_empty() {
        return Ok(RenderConfig::default());
    }
    serde_json::from_str(raw).map_err(|e| format!("failed to decode config JSON: {e}"))
}

fn finish_result(result: Result<String, String>, out_is_error: *mut c_int) -> *mut c_char {
    if out_is_error.is_null() {
        return ptr::null_mut();
    }

    match result {
        Ok(output) => {
            unsafe { *out_is_error = 0 };
            CString::new(output)
                .map(CString::into_raw)
                .unwrap_or(ptr::null_mut())
        }
        Err(message) => {
            unsafe { *out_is_error = 1 };
            CString::new(message)
                .map(CString::into_raw)
                .unwrap_or(ptr::null_mut())
        }
    }
}

fn run_render<F>(out_is_error: *mut c_int, f: F) -> *mut c_char
where
    F: FnOnce() -> Result<String, String>,
{
    let result = catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|panic| {
        Err(format!(
            "panic while rendering query plan: {}",
            panic_to_string(panic)
        ))
    });
    finish_result(result, out_is_error)
}

fn panic_to_string(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

/// Renders a query plan from protobuf wire bytes.
///
/// Returns a NUL-terminated UTF-8 string that must be freed with
/// [`spannerplan_string_free`]. Returns NULL on allocation failure. On render
/// error, `*out_is_error` is set to 1 and the returned string holds the message.
///
/// # Safety
///
/// `plan_wire` must point to `plan_wire_len` valid bytes when non-null.
/// `mode`, `format`, and `config_json` (if non-null) must be valid NUL-terminated
/// UTF-8. `out_is_error` must be non-null.
#[no_mangle]
pub unsafe extern "C" fn spannerplan_render_tree_table_wire(
    plan_wire: *const u8,
    plan_wire_len: usize,
    mode: *const c_char,
    format: *const c_char,
    config_json: *const c_char,
    out_is_error: *mut c_int,
) -> *mut c_char {
    run_render(out_is_error, || {
        if plan_wire.is_null() && plan_wire_len != 0 {
            return Err("plan_wire is null".to_string());
        }
        let mode = read_cstr(mode, "mode")?;
        let format = read_cstr(format, "format")?;
        let bytes = if plan_wire_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(plan_wire, plan_wire_len) }
        };
        let plan_nodes = wire::decode_plan_nodes(bytes).map_err(|e| e.to_string())?;
        render_internal(plan_nodes, mode, format, config_json)
    })
}

/// Renders a query plan from JSON/YAML text (QueryPlan, ResultSetStats, or
/// ResultSet shapes).
///
/// # Safety
///
/// `plan_json`, `mode`, and `format` must be valid NUL-terminated UTF-8.
/// `config_json`, if non-null, must likewise be valid UTF-8. `out_is_error`
/// must be non-null.
#[no_mangle]
pub unsafe extern "C" fn spannerplan_render_tree_table_json(
    plan_json: *const c_char,
    mode: *const c_char,
    format: *const c_char,
    config_json: *const c_char,
    out_is_error: *mut c_int,
) -> *mut c_char {
    run_render(out_is_error, || {
        let plan_json = read_cstr(plan_json, "plan_json")?;
        let mode = read_cstr(mode, "mode")?;
        let format = read_cstr(format, "format")?;
        let plan_nodes = extract_plan_nodes(plan_json.as_bytes()).map_err(|e| e.to_string())?;
        render_internal(plan_nodes, mode, format, config_json)
    })
}

/// Frees a string returned by the render entry points.
///
/// # Safety
///
/// `s` must be NULL or a pointer previously returned by a render entry point
/// and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn spannerplan_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}

fn read_cstr<'a>(ptr: *const c_char, name: &str) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err(format!("{name} is null"));
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|e| format!("{name} is not valid UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn call_json(plan: &str, mode: &str, format: &str, config: Option<&str>) -> (String, i32) {
        let plan_c = CString::new(plan).unwrap();
        let mode_c = CString::new(mode).unwrap();
        let format_c = CString::new(format).unwrap();
        let config_c = config.map(CString::new).transpose().unwrap();
        let mut is_error = 0;
        let out = unsafe {
            spannerplan_render_tree_table_json(
                plan_c.as_ptr(),
                mode_c.as_ptr(),
                format_c.as_ptr(),
                config_c.as_ref().map_or(ptr::null(), |c| c.as_ptr()),
                &mut is_error,
            )
        };
        assert!(!out.is_null());
        let s = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_string();
        unsafe { spannerplan_string_free(out) };
        (s, is_error)
    }

    #[test]
    fn json_entry_point_renders_fixture() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/reference/dca.yaml"
        );
        let plan = std::fs::read_to_string(path).unwrap();
        let (output, is_error) = call_json(&plan, "AUTO", "CURRENT", None);
        assert_eq!(is_error, 0);
        assert!(output.contains("Distributed Cross Apply"));
    }

    #[test]
    fn json_entry_point_null_plan_json_sets_error() {
        let mode_c = CString::new("AUTO").unwrap();
        let format_c = CString::new("CURRENT").unwrap();
        let mut is_error = 0;
        let out = unsafe {
            spannerplan_render_tree_table_json(
                ptr::null(),
                mode_c.as_ptr(),
                format_c.as_ptr(),
                ptr::null(),
                &mut is_error,
            )
        };
        assert!(!out.is_null());
        let message = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
        assert_eq!(is_error, 1);
        assert!(message.contains("plan_json is null"));
        unsafe { spannerplan_string_free(out) };
    }

    #[test]
    fn json_entry_point_invalid_mode_sets_error() {
        let (message, is_error) =
            call_json(r#"{"planNodes": [{"index": 0}]}"#, "NOT_A_MODE", "CURRENT", None);
        assert_eq!(is_error, 1);
        assert!(!message.is_empty());
    }

    #[test]
    fn json_entry_point_malformed_input_sets_error() {
        let (message, is_error) = call_json("not a plan document", "AUTO", "CURRENT", None);
        assert_eq!(is_error, 1);
        assert!(!message.is_empty());
    }

    #[test]
    fn wire_entry_point_renders_fixture() {
        use prost::Message;

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/reference/dca.yaml"
        );
        let bytes = std::fs::read(path).unwrap();
        let json_nodes = extract_plan_nodes(&bytes).unwrap();
        let wire_plan = wire::encode_query_plan_for_test(&json_nodes);
        let wire_bytes = wire_plan.encode_to_vec();

        let mode_c = CString::new("AUTO").unwrap();
        let format_c = CString::new("CURRENT").unwrap();
        let mut is_error = 0;
        let out = unsafe {
            spannerplan_render_tree_table_wire(
                wire_bytes.as_ptr(),
                wire_bytes.len(),
                mode_c.as_ptr(),
                format_c.as_ptr(),
                ptr::null(),
                &mut is_error,
            )
        };
        assert!(!out.is_null());
        let output = unsafe { CStr::from_ptr(out) }.to_str().unwrap();
        assert_eq!(is_error, 0);
        assert!(output.contains("Distributed Cross Apply"));
        unsafe { spannerplan_string_free(out) };
    }

    #[test]
    fn committed_header_matches_cbindgen() {
        let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let committed =
            std::fs::read_to_string(crate_dir.join("spannerplan.h")).expect("read spannerplan.h");
        let bindings = cbindgen::Builder::new()
            .with_crate(crate_dir)
            .with_language(cbindgen::Language::C)
            .with_include_guard("SPANNERPLAN_H")
            .generate()
            .expect("generate C header");
        let mut generated = Vec::new();
        bindings.write(&mut generated);
        let generated = String::from_utf8(generated).expect("header UTF-8");
        assert_eq!(
            committed, generated,
            "spannerplan.h is out of date; regenerate with:\n  \
             cbindgen --crate crates/spannerplan-ffi --lang c --guard SPANNERPLAN_H \
             --output crates/spannerplan-ffi/spannerplan.h"
        );
    }
}
