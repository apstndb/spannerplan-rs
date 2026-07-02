//! Size probe: host passes decoded `PlanNode[]`; WASM only renders.

use serde::Serialize;
use spannerplan_core::model::PlanNode;
use spannerplan_core::reference::{
    parse_format, parse_render_mode, render_tree_table_with_config, RenderConfig,
};
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[wasm_bindgen(js_name = spannerplanRenderTreeTable)]
pub fn spannerplan_render_tree_table(args: JsValue) -> JsValue {
    let args: Vec<JsValue> = js_sys::Array::from(&args).iter().collect();
    let result = (|| -> Result<RenderResponse, String> {
        let plan = args
            .first()
            .ok_or_else(|| "planNodes argument is required".to_string())?;
        let plan_nodes: Vec<PlanNode> = serde_wasm_bindgen::from_value(plan.clone())
            .map_err(|e| format!("decode planNodes: {e}"))?;
        let mode = optional_string(&args, 1, "AUTO")?;
        let format = optional_string(&args, 2, "CURRENT")?;
        let config = if args.len() > 3 {
            decode_config(args[3].clone())?
        } else {
            RenderConfig::default()
        };
        match (parse_render_mode(&mode), parse_format(&format)) {
            (Ok(mode), Ok(format)) => match render_tree_table_with_config(
                &plan_nodes,
                mode,
                format,
                &config,
            ) {
                Ok(output) => Ok(RenderResponse {
                    output: Some(output),
                    error: None,
                }),
                Err(err) => Ok(RenderResponse {
                    output: None,
                    error: Some(err.to_string()),
                }),
            },
            (Err(err), _) | (_, Err(err)) => Ok(RenderResponse {
                output: None,
                error: Some(err.to_string()),
            }),
        }
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

fn optional_string(args: &[JsValue], index: usize, fallback: &str) -> Result<String, String> {
    if index >= args.len() || args[index].is_undefined() || args[index].is_null() {
        return Ok(fallback.to_string());
    }
    args[index]
        .as_string()
        .ok_or_else(|| "expected string argument".to_string())
}

fn decode_config(config: JsValue) -> Result<RenderConfig, String> {
    if config.is_undefined() || config.is_null() {
        return Ok(RenderConfig::default());
    }
    serde_wasm_bindgen::from_value(config).map_err(|e| format!("decode config: {e}"))
}
