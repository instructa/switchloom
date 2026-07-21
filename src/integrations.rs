use crate::contracts::*;
use serde_json::{Value, json};

pub(crate) fn render_planr_native_role(artifact: &SourceArtifact) -> String {
    let protocol = if is_reviewer_role(artifact) {
        Some((
            "$planr-review",
            "Use the existing Planr internal review protocol for exactly one Planr review item. Read the pick packet, audit the target item and evidence, report findings first, and return the review verdict through Planr. Do not create or invoke any routing-specific, goal, or loop workflow skill. Planr users enter only through $planr-goal and $planr-loop.",
        ))
    } else if is_worker_role(artifact) {
        Some((
            "$planr-work",
            "Use the existing Planr internal worker protocol for exactly one picked Planr item. Read the pick packet, implement only that item, log changed files and real verification commands, request review through Planr, and stop. Do not create or invoke any routing-specific, goal, or loop workflow skill. Planr users enter only through $planr-goal and $planr-loop.",
        ))
    } else {
        None
    };
    let Some((protocol_name, instructions)) = protocol else {
        return artifact.content.clone();
    };
    if artifact.path.starts_with(".pi/workflows/") {
        rewrite_json_workflow_protocol_preload(&artifact.content, protocol_name, instructions)
    } else if artifact.path.starts_with(".codex/agents/") {
        rewrite_codex_developer_instructions(&artifact.content, protocol_name, instructions)
    } else {
        rewrite_markdown_agent_body(&artifact.content, protocol_name, instructions)
    }
}

#[cfg(test)]
#[path = "tests/integrations.rs"]
mod tests;

pub(crate) fn is_worker_role(artifact: &SourceArtifact) -> bool {
    artifact.path.contains("terra-high")
        || artifact.path.contains("luna-xhigh")
        || artifact.path.contains("preset-worker")
        || artifact.path.starts_with(".pi/workflows/")
        || artifact.path.contains("implementer")
        || artifact.content.contains("Normal implementation")
        || artifact.content.contains("Bounded checklist")
        || artifact.content.contains("custom implementer role")
}

pub(crate) fn is_reviewer_role(artifact: &SourceArtifact) -> bool {
    artifact.path.contains("sol-high")
        || artifact.path.contains("reviewer")
        || artifact.path.contains("verifier")
        || artifact.content.contains("Independent final review")
        || artifact.content.contains("custom reviewer role")
        || artifact.content.contains("custom verifier role")
}

pub(crate) fn rewrite_codex_developer_instructions(
    content: &str,
    protocol_name: &str,
    instructions: &str,
) -> String {
    let marker = "developer_instructions = \"\"\"\n";
    if let Some(start) = content.find(marker) {
        let body_start = start + marker.len();
        if let Some(end) = content[body_start..].find("\n\"\"\"") {
            let body_end = body_start + end;
            let mut output = String::new();
            output.push_str(&content[..body_start]);
            output.push_str(instructions);
            output.push_str("\n\nProtocol preload: ");
            output.push_str(protocol_name);
            output.push_str(&content[body_end..]);
            return output;
        }
    }
    format!("{content}\n\nProtocol preload: {protocol_name}\n{instructions}\n")
}

pub(crate) fn rewrite_markdown_agent_body(
    content: &str,
    protocol_name: &str,
    instructions: &str,
) -> String {
    if let Some(rest) = content.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let split = "---\n".len() + end + "\n---\n".len();
            return format!(
                "{}Protocol preload: {}\n\n{}\n",
                &content[..split],
                protocol_name,
                instructions
            );
        }
    }
    format!("Protocol preload: {protocol_name}\n\n{instructions}\n")
}

pub(crate) fn rewrite_json_workflow_protocol_preload(
    content: &str,
    protocol_name: &str,
    instructions: &str,
) -> String {
    let mut value: Value = serde_json::from_str(content).unwrap_or_else(|_| json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "protocol_preload".to_string(),
            json!({
                "marker": format!("Protocol preload: {protocol_name}"),
                "instructions": instructions
            }),
        );
    }
    let mut output = serde_json::to_string_pretty(&value).unwrap_or_else(|_| {
        format!("{content}\n\nProtocol preload: {protocol_name}\n{instructions}\n")
    });
    output.push('\n');
    output
}
