import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
test("scrubbed smoke correlates parent and three child routes", async () => { const out="/private/tmp/pilot-smoke-test.json", run=spawnSync(process.execPath,["scripts/extract-routing-pilot-telemetry.mjs","--thread-id","019f93fa-c29c-7923-a79c-7b80a2477c17","--input","fixtures/routing-pilot-v1/smoke-parent.jsonl","--session","fixtures/routing-pilot-v1/smoke-children.jsonl","--output",out],{encoding:"utf8"}); assert.equal(run.status,0,run.stderr); const summary=JSON.parse(await readFile(out,"utf8")); assert.equal(summary.sessions.length,4); const byRole=Object.fromEntries(summary.sessions.map((s)=>[s.agent_role??"parent",s])); assert.equal(byRole.parent.model,"gpt-5.6-sol"); assert.equal(byRole.parent.usage.input_tokens,189116); assert.equal(byRole.pilot_implementer.usage.input_tokens,94461); assert.equal(byRole.pilot_reviewer.usage.input_tokens,54972); assert.equal(byRole.pilot_verifier.effort,"low"); assert.equal(byRole.pilot_verifier.usage.reasoning_output_tokens,49); });
test("telemetry extractor preserves unavailable fields and forbids currency claims", async (t) => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "routing-pilot-")); t.after(() => rm(directory, { force: true, recursive: true }));
  const input = path.join(directory, "events.jsonl"), output = path.join(directory, "summary.json");
  await writeFile(input, `${JSON.stringify({ type:"turn_context", thread_id:"parent", payload:{model:"gpt-5.6-sol",effort:"medium"} })}\n${JSON.stringify({ type:"turn.completed", thread_id:"parent", usage:{input_tokens:10,cached_input_tokens:2,output_tokens:3,reasoning_output_tokens:1} })}\n${JSON.stringify({ type:"item.started", thread_id:"parent", item:{type:"tool_call"} })}\n`);
  const run = spawnSync(process.execPath, ["scripts/extract-routing-pilot-telemetry.mjs", "--input", input, "--output", output, "--thread-id", "parent"], { encoding:"utf8" });
  assert.equal(run.status, 0, run.stderr); const summary = JSON.parse(await readFile(output, "utf8"));
  assert.equal(summary.currency.availability, "unavailable"); assert.equal(summary.sessions[0].usage.input_tokens, 10); assert.equal(summary.sessions[0].rework, null); assert.equal(summary.sessions[0].availability.rework, "unavailable"); assert.equal(summary.sessions[0].availability.quality_outcome, "unavailable"); assert.equal(summary.sessions[0].elapsed_ms, null);
});
test("telemetry records observed retry, rejection, and tool evidence", async (t) => {
  const directory=await mkdtemp(path.join(os.tmpdir(),"routing-pilot-events-")); t.after(()=>rm(directory,{force:true,recursive:true}));
  const input=path.join(directory,"events.jsonl"), stdout=path.join(directory,"stdout.jsonl"), stderr=path.join(directory,"stderr.txt"), output=path.join(directory,"summary.json");
  await writeFile(input,`${JSON.stringify({timestamp:"2026-07-24T12:00:00.000Z",type:"turn_context",thread_id:"parent",payload:{model:"gpt-5.6-sol",effort:"medium"}})}\n${JSON.stringify({timestamp:"2026-07-24T12:00:02.500Z",type:"turn.completed",thread_id:"parent",usage:{input_tokens:1}})}\n`);
  await writeFile(stdout,`${JSON.stringify({type:"item.started",item:{type:"command_execution"}})}\n${JSON.stringify({type:"item.started",item:{type:"collab_tool_call",tool:"spawn_agent"}})}\n`);
  await writeFile(stderr,"ERROR codex_core::tools::router: error=Full-history forked agents inherit the parent agent type; omit agent_type\nWARN codex_core::responses_retry: retrying sampling request (1/5)\nERROR codex_core::tools::router: error=exec_command failed for command: CreateProcess { message: \"Rejected(unsafe)\" }\nWARN codex_rmcp_client::rmcp_client::streamable_http_retry: streamable HTTP MCP initialize failed with a retryable error; retrying attempt=1\nERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed\nWARN codex_analytics::client: failed to send events request: error sending request\n");
  const run=spawnSync(process.execPath,["scripts/extract-routing-pilot-telemetry.mjs","--input",input,"--output",output,"--thread-id","parent","--stdout",stdout,"--stderr",stderr],{encoding:"utf8"});
  assert.equal(run.status,0,run.stderr); const summary=JSON.parse(await readFile(output,"utf8"));
  assert.equal(summary.sessions[0].elapsed_ms,2500); assert.equal(summary.operational_events.sampling_retries.count,1); assert.equal(summary.operational_events.rejected_spawn_attempts.count,1); assert.equal(summary.operational_events.command_rejections.count,1); assert.equal(summary.operational_events.mcp_initialize_retries.count,1); assert.equal(summary.operational_events.transport_failures.count,1); assert.equal(summary.operational_events.analytics_failures.count,1); assert.equal(summary.operational_events.tool_activity.command_executions,1); assert.equal(summary.operational_events.tool_activity.collaboration_calls,1);
});
test("exec-command policy rejection is not a spawn rejection", async (t) => {
  const directory=await mkdtemp(path.join(os.tmpdir(),"routing-pilot-exclusive-")); t.after(()=>rm(directory,{force:true,recursive:true}));
  const input=path.join(directory,"events.jsonl"), stderr=path.join(directory,"stderr.txt"), output=path.join(directory,"summary.json");
  await writeFile(input,`${JSON.stringify({type:"turn_context",thread_id:"parent",payload:{model:"gpt-5.6-sol",effort:"medium"}})}\n${JSON.stringify({type:"turn.completed",thread_id:"parent",usage:{input_tokens:1}})}\n`);
  await writeFile(stderr,"ERROR codex_core::tools::router: error=Full-history forked agents inherit the parent agent type; omit agent_type\nERROR codex_core::tools::router: error=exec_command failed for command: CreateProcess { message: \"Rejected(policy)\" }\n");
  const run=spawnSync(process.execPath,["scripts/extract-routing-pilot-telemetry.mjs","--input",input,"--output",output,"--thread-id","parent","--stderr",stderr],{encoding:"utf8"}); assert.equal(run.status,0,run.stderr);
  const events=JSON.parse(await readFile(output,"utf8")).operational_events; assert.equal(events.rejected_spawn_attempts.count,1); assert.equal(events.command_rejections.count,1);
});
test("command output and agent prose cannot create operational routing signals", async (t) => {
  const directory=await mkdtemp(path.join(os.tmpdir(),"routing-pilot-untrusted-")); t.after(()=>rm(directory,{force:true,recursive:true}));
  const input=path.join(directory,"events.jsonl"), stdout=path.join(directory,"stdout.jsonl"), stderr=path.join(directory,"stderr.txt"), output=path.join(directory,"summary.json");
  await writeFile(input,`${JSON.stringify({type:"turn_context",thread_id:"parent",payload:{model:"gpt-5.6-sol",effort:"medium"}})}\n${JSON.stringify({type:"turn.completed",thread_id:"parent",usage:{input_tokens:1}})}\n`);
  await writeFile(stdout,`${JSON.stringify({type:"item.completed",item:{type:"command_execution",aggregated_output:"fallback rejected spawn failure unknown agent thread limit"}})}\n${JSON.stringify({type:"item.completed",item:{type:"agent_message",text:"I saw a fallback and rejected spawn failure."}})}\n`);
  await writeFile(stderr,"");
  const run=spawnSync(process.execPath,["scripts/extract-routing-pilot-telemetry.mjs","--input",input,"--output",output,"--thread-id","parent","--stdout",stdout,"--stderr",stderr],{encoding:"utf8"}); assert.equal(run.status,0,run.stderr);
  const events=JSON.parse(await readFile(output,"utf8")).operational_events;
  assert.equal(events.rejected_spawn_attempts.count,0); assert.equal(events.fallback_signals.count,0); assert.equal(events.unknown_agent_signals.count,0); assert.equal(events.thread_limit_signals.count,0); assert.equal(events.tool_activity.command_executions,1);
});
