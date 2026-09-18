//! JSON-only web boundary; all validation and routing remain in the native core.
use crate::{
    catalog::Profile,
    decision::{RouteRequest, finish_route, prepare_route},
};
use serde::Deserialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request: RouteRequest,
    profiles: BTreeMap<String, Profile>,
}
fn input(json: &str) -> Result<Input, JsError> {
    if json.len() > 65_536 {
        return Err(JsError::new("routing input exceeds 65536 bytes"));
    }
    Ok(serde_json::from_str(json)?)
}
#[wasm_bindgen]
pub fn prepare(json: &str) -> Result<String, JsError> {
    let input = input(json)?;
    Ok(serde_json::to_string(&prepare_route(
        &input.request,
        &input.profiles,
    )?)?)
}
#[wasm_bindgen]
pub fn complete(json: &str, response: &str) -> Result<String, JsError> {
    let input = input(json)?;
    if response.len() > 65_536 {
        return Err(JsError::new("TypeSafe response exceeds 65536 bytes"));
    }
    Ok(serde_json::to_string(&finish_route(
        &input.request,
        &input.profiles,
        serde_json::from_str(response)?,
    )?)?)
}
