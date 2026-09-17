//! Bind a routing decision to an existing Codex desktop task.
//! Codex owns dispatch and result messages; there is no waiting runtime here.

use crate::catalog::Profile;
use crate::decision::{RouteDecision, RouteRequest, route_task_for_profiles};
use crate::error::Result;
use crate::{bail, product_error};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_HANDOFF_INPUT_BYTES: usize = 65_536;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffRequest {
    pub request: RouteRequest,
    pub caller: TaskTarget,
    pub threads: BTreeMap<String, ModelTask>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct TaskTarget {
    pub thread_id: String,
    pub host_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTask {
    pub thread_id: String,
    pub host_id: String,
    pub model: String,
    pub effort: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dispatch {
    pub thread_id: String,
    pub host_id: String,
    pub model: String,
    pub thinking: String,
    pub prompt: String,
}

#[derive(Debug, Serialize)]
pub struct Handoff {
    pub decision: RouteDecision,
    pub action: HandoffAction,
    pub dispatch: Option<Dispatch>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandoffAction {
    Dispatch,
    ContinueHere,
    Clarify,
}

pub fn prepare_handoff(input: &HandoffRequest) -> Result<Handoff> {
    validate_targets(input)?;
    let settings = input
        .threads
        .iter()
        .map(|(owner, task)| {
            (
                owner.clone(),
                Profile {
                    capabilities: task.capabilities.clone(),
                    model: task.model.clone(),
                    effort: task.effort.clone(),
                },
            )
        })
        .collect();
    bind_decision(input, route_task_for_profiles(&input.request, &settings)?)
}

fn validate_targets(input: &HandoffRequest) -> Result<()> {
    let valid_id =
        |id: &str| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_whitespace);
    let addresses = input
        .threads
        .values()
        .map(|task| (&task.thread_id, &task.host_id));
    for (thread, host) in
        std::iter::once((&input.caller.thread_id, &input.caller.host_id)).chain(addresses.clone())
    {
        if !valid_id(thread) || !valid_id(host) {
            bail!("task and host IDs must be nonblank identifiers of at most 256 bytes");
        }
    }
    if input.threads.is_empty() || addresses.collect::<BTreeSet<_>>().len() != input.threads.len() {
        bail!("provide distinct existing tasks for the configured models");
    }
    Ok(())
}

fn bind_decision(input: &HandoffRequest, decision: RouteDecision) -> Result<Handoff> {
    let dispatch = if let Some(assignment) = &decision.assignment {
        let target = input.threads.get(&assignment.owner).ok_or_else(|| {
            product_error!("selected owner has no task; supply its existing task ID")
        })?;
        if target.thread_id == input.caller.thread_id && target.host_id == input.caller.host_id {
            return Ok(Handoff {
                decision,
                action: HandoffAction::ContinueHere,
                dispatch: None,
            });
        }
        let return_target = serde_json::json!({
            "threadId": input.caller.thread_id,
            "hostId": input.caller.host_id,
        });
        let prompt = format!(
            "Assignment:\n{}\n\nCapability:\n{}\n\nRelevant context and ownership:\n{}\n\nReturn target: {}\n\nOn completion, a blocker, or a decision needed to proceed, use send_message_to_thread to send one substantive result to the return target. Start it with 'Result for:' and the assignment objective; identify your thread as {} on host {}. Include the diff or relevant files, evidence, remaining limitations and requested next action. Preserve the recipient's model settings. Then end your turn. Do not poll, wait, send acknowledgments or progress-only messages. A result is a continuation of this assignment, not a new assignment to echo back. Create no additional tasks or subagents for this step.",
            input.request.task,
            assignment.instructions,
            input.request.context,
            return_target,
            target.thread_id,
            target.host_id,
        );
        Some(Dispatch {
            thread_id: target.thread_id.clone(),
            host_id: target.host_id.clone(),
            model: assignment.model.clone(),
            thinking: assignment.effort.clone(),
            prompt,
        })
    } else {
        None
    };
    let action = if dispatch.is_some() {
        HandoffAction::Dispatch
    } else {
        HandoffAction::Clarify
    };
    Ok(Handoff {
        decision,
        action,
        dispatch,
    })
}

#[cfg(test)]
#[path = "tests/handoff.rs"]
mod tests;
