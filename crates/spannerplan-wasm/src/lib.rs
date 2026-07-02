//! WASM bindings mirroring the Go `examples/wasm/render` wrapper.

use serde::Serialize;
use spannerplan::core::reference::{
    parse_format, parse_render_mode, render_tree_table_with_config, RenderConfig,
};
use spannerplan::core::wire;
use spannerplan::extract::extract_plan_nodes;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn render_from_plan_nodes(
    plan_nodes: Vec<spannerplan::core::model::PlanNode>,
    mode: &str,
    format: &str,
    config: RenderConfig,
) -> RenderResponse {
    match (parse_render_mode(mode), parse_format(format)) {
        (Ok(mode), Ok(format)) => {
            match render_tree_table_with_config(&plan_nodes, mode, format, &config) {
                Ok(output) => RenderResponse {
                    output: Some(output),
                    error: None,
                },
                Err(err) => RenderResponse {
                    output: None,
                    error: Some(err.to_string()),
                },
            }
        }
        (Err(err), _) | (_, Err(err)) => RenderResponse {
            output: None,
            error: Some(err.to_string()),
        },
    }
}

fn decode_config(config: JsValue) -> Result<RenderConfig, String> {
    if config.is_undefined() || config.is_null() {
        return Ok(RenderConfig::default());
    }
    serde_wasm_bindgen::from_value(config).map_err(|e| format!("decode config JSON: {e}"))
}

fn decode_plan_json(plan: JsValue) -> Result<Vec<spannerplan::core::model::PlanNode>, String> {
    if plan.is_undefined() || plan.is_null() {
        return Err("query plan argument is required".to_string());
    }

    if plan.is_string() {
        let text = plan
            .as_string()
            .ok_or_else(|| "query plan string is not valid UTF-8".to_string())?;
        return extract_plan_nodes(text.as_bytes()).map_err(|e| e.to_string());
    }

    if js_sys::Uint8Array::instanceof(&plan) {
        let bytes = js_sys::Uint8Array::new(&plan).to_vec();
        return wire::decode_plan_nodes(&bytes).map_err(|e| e.to_string());
    }

    if plan.is_object() {
        let value: serde_json::Value = serde_wasm_bindgen::from_value(plan)
            .map_err(|e| format!("decode query plan argument: {e}"))?;
        let text = serde_json::to_string(&value)
            .map_err(|e| format!("stringify query plan argument: {e}"))?;
        return extract_plan_nodes(text.as_bytes()).map_err(|e| e.to_string());
    }

    Err(format!(
        "expected JSON string, object, or Uint8Array, got {}",
        plan.js_typeof().as_string().unwrap_or_default()
    ))
}

fn optional_string_arg(
    args: &[JsValue],
    index: usize,
    fallback: &str,
    name: &str,
) -> Result<String, String> {
    if index >= args.len() || args[index].is_undefined() || args[index].is_null() {
        return Ok(fallback.to_string());
    }
    args[index]
        .as_string()
        .ok_or_else(|| format!("{name} must be a string"))
}

#[wasm_bindgen(js_name = spannerplanRenderTreeTable)]
pub fn spannerplan_render_tree_table(args: JsValue) -> JsValue {
    let args: Vec<JsValue> = js_sys::Array::from(&args).iter().collect();
    let result = (|| -> Result<RenderResponse, String> {
        let plan = args
            .first()
            .ok_or_else(|| "query plan argument is required".to_string())?;
        let plan_nodes = decode_plan_json(plan.clone())?;
        let mode = optional_string_arg(&args, 1, "AUTO", "mode")?;
        let format = optional_string_arg(&args, 2, "CURRENT", "format")?;
        let config = if args.len() > 3 {
            decode_config(args[3].clone())?
        } else {
            RenderConfig::default()
        };
        Ok(render_from_plan_nodes(plan_nodes, &mode, &format, config))
    })();

    match result {
        Ok(response) => serde_wasm_bindgen::to_value(&response).unwrap_or_else(|err| {
            serde_wasm_bindgen::to_value(&RenderResponse {
                output: None,
                error: Some(format!("failed to serialize response: {err}")),
            })
            .unwrap()
        }),
        Err(error) => serde_wasm_bindgen::to_value(&RenderResponse {
            output: None,
            error: Some(error),
        })
        .unwrap(),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RendertreeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
}

/// `rendertree` CLI rendering: stdin bytes + flag argv → stdout (matches Go/Rust CLI).
#[wasm_bindgen(js_name = spannerplanRenderRendertree)]
pub fn spannerplan_render_rendertree(input: js_sys::Uint8Array, args: JsValue) -> JsValue {
    let result = (|| -> Result<RendertreeResponse, String> {
        let mut bytes = vec![0u8; input.length() as usize];
        input.copy_to(&mut bytes);
        let arg_list: Vec<String> = if args.is_undefined() || args.is_null() {
            Vec::new()
        } else {
            serde_wasm_bindgen::from_value(args)
                .map_err(|e| format!("decode args: {e}"))?
        };
        let arg_refs: Vec<&str> = arg_list.iter().map(String::as_str).collect();
        match spannerplan_cli::run_collecting(&arg_refs, &bytes) {
            Ok(spannerplan_cli::RunCollectResult::Rendered { stdout, stderr }) => {
                Ok(RendertreeResponse {
                    output: Some(stdout),
                    stderr: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                    error: None,
                    kind: Some("rendered".into()),
                })
            }
            Ok(spannerplan_cli::RunCollectResult::Help { stderr }) => Ok(RendertreeResponse {
                output: None,
                stderr: Some(stderr),
                error: None,
                kind: Some("help".into()),
            }),
            Err(spannerplan_cli::RunCollectError::Usage { stderr, message }) => {
                Ok(RendertreeResponse {
                    output: None,
                    stderr: Some(stderr),
                    error: Some(message),
                    kind: Some("usage".into()),
                })
            }
            Err(spannerplan_cli::RunCollectError::Failed { stderr, message }) => {
                Ok(RendertreeResponse {
                    output: None,
                    stderr: Some(stderr),
                    error: Some(message),
                    kind: Some("failed".into()),
                })
            }
        }
    })();

    match result {
        Ok(response) => serde_wasm_bindgen::to_value(&response).unwrap(),
        Err(error) => serde_wasm_bindgen::to_value(&RendertreeResponse {
            output: None,
            stderr: None,
            error: Some(error),
            kind: Some("failed".into()),
        })
        .unwrap(),
    }
}

/// Wire-bytes variant for callers that already hold protobuf-encoded plan data.
#[wasm_bindgen(js_name = spannerplanRenderTreeTableWire)]
pub fn spannerplan_render_tree_table_wire(
    plan_wire: js_sys::Uint8Array,
    mode: Option<String>,
    format: Option<String>,
    config: JsValue,
) -> JsValue {
    let result = (|| -> Result<RenderResponse, String> {
        let mut bytes = vec![0u8; plan_wire.length() as usize];
        plan_wire.copy_to(&mut bytes);
        let plan_nodes = wire::decode_plan_nodes(&bytes).map_err(|e| e.to_string())?;
        let mode = mode.unwrap_or_else(|| "AUTO".to_string());
        let format = format.unwrap_or_else(|| "CURRENT".to_string());
        let config = decode_config(config)?;
        Ok(render_from_plan_nodes(plan_nodes, &mode, &format, config))
    })();

    match result {
        Ok(response) => serde_wasm_bindgen::to_value(&response).unwrap(),
        Err(error) => serde_wasm_bindgen::to_value(&RenderResponse {
            output: None,
            error: Some(error),
        })
        .unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_response_serializes_like_go_wrapper() {
        let response = RenderResponse {
            output: Some("ok".into()),
            error: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, r#"{"output":"ok"}"#);
    }
}
