//! TypeSafe HTTP boundary. Product routing policy lives in `decision`.

#[cfg(not(target_arch = "wasm32"))]
use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use crate::{bail, product_error};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::{io::Read, time::Duration};

#[cfg(not(target_arch = "wasm32"))]
const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
#[cfg(not(target_arch = "wasm32"))]
const RESPONSE_LIMIT: u64 = 65_536;

#[derive(Debug, Deserialize)]
pub(crate) struct Judgment {
    pub model: String,
    pub answers: Answers,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Answers {
    pub route: ChoiceAnswer,
    pub context_missing: NoulAnswer,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChoiceAnswer {
    #[serde(rename = "type")]
    pub kind: String,
    pub choice: String,
    pub confidence: f64,
    pub probabilities: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NoulAnswer {
    #[serde(rename = "type")]
    pub kind: String,
    pub noul: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn evaluate(state: Value, questions: Value) -> Result<Judgment> {
    let key = std::env::var("TYPESAFE_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| product_error!("set TYPESAFE_API_KEY to use Jev routing"))?;
    send(
        ENDPOINT,
        &key,
        request_body(state, questions),
        Duration::from_secs(5),
    )
}

pub(crate) fn request_body(state: Value, questions: Value) -> Value {
    json!({
        "model": crate::catalog::JUDGE_MODEL,
        "state": state,
        "questions": questions,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn send(endpoint: &str, key: &str, body: Value, timeout: Duration) -> Result<Judgment> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .build()
        .into();
    let response = agent.post(endpoint)
        .header("Authorization", &format!("Bearer {key}"))
        .send_json(body)
        .map_err(|error| match error {
            ureq::Error::StatusCode(status) => product_error!("TypeSafe returned HTTP {status}; no routing decision was made"),
            _ => product_error!("TypeSafe could not be reached within the request deadline; no routing decision was made"),
        })?;
    if !(200..300).contains(&response.status().as_u16()) {
        bail!(
            "TypeSafe returned HTTP {}; no routing decision was made",
            response.status().as_u16()
        );
    }
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(RESPONSE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| product_error!("could not read TypeSafe response"))?;
    if bytes.len() as u64 > RESPONSE_LIMIT {
        bail!("TypeSafe response exceeds the size limit");
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| product_error!("TypeSafe returned an invalid routing response"))
}

#[cfg(test)]
#[path = "tests/typesafe.rs"]
mod tests;
