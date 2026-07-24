#!/usr/bin/env node
import { readFile, writeFile } from "node:fs/promises";

const args = process.argv.slice(2);
const value = (flag) => { const index=args.indexOf(flag); return index < 0 ? null : args[index + 1]; };
const input=value("--input"), output=value("--output"), currentThreadId=value("--thread-id") ?? null;
if (!input || !output) throw new Error("usage: extract-routing-pilot-telemetry.mjs --input events.jsonl --output summary.json");
const sessionPaths=args.flatMap((arg,index)=>arg==="--session"?[args[index+1]]:[]).filter(Boolean);
const stdoutPath=value("--stdout"), stderrPath=value("--stderr");
const parseRecords=async (file) => {
  const text=await readFile(file,"utf8"); let fileSession=currentThreadId;
  return text.split("\n").filter(Boolean).flatMap((line)=>{
    try {
      const record=JSON.parse(line);
      if(record.type==="session_meta") fileSession=record.payload?.id ?? record.payload?.session_id ?? fileSession;
      return [{...record,__fileSession:fileSession,__timestamp:record.timestamp ?? record.payload?.timestamp ?? null}];
    } catch { return []; }
  });
};
const records=(await Promise.all([input,...sessionPaths].map(parseRecords))).flat();
const stdout=stdoutPath ? await readFile(stdoutPath,"utf8").catch(()=>null) : null;
const stderr=stderrPath ? await readFile(stderrPath,"utf8").catch(()=>null) : null;
const count=(text,pattern)=>text ? [...text.matchAll(pattern)].length : 0;
const stdoutRecords=stdout ? stdout.split("\n").filter(Boolean).flatMap((line)=>{ try { return [JSON.parse(line)]; } catch { return []; } }) : [];
const itemCount=(type)=>stdoutRecords.filter((record)=>record.item?.type===type).length;
const samplingRetries=count(stderr,/retrying sampling request/gi);
const stderrRejectedSpawns=count(stderr,/codex_core::tools::router: error=Full-history forked agents inherit the parent agent type;/g);
const stderrCommandRejections=count(stderr,/codex_core::tools::router: error=exec_command failed[\s\S]*?CreateProcess \{ message: "Rejected\(/g);
const operational_events={
  sampling_retries:{count:samplingRetries,availability:stderr===null?"unavailable":"observed"},
  rejected_spawn_attempts:{count:stderrRejectedSpawns,availability:stderr===null?"unavailable":"observed"},
  command_rejections:{count:stderrCommandRejections,availability:stderr===null?"unavailable":"observed"},
  mcp_initialize_retries:{count:count(stderr,/codex_rmcp_client::rmcp_client::streamable_http_retry: streamable HTTP MCP initialize failed with a retryable error; retrying attempt=/g),availability:stderr===null?"unavailable":"observed"},
  transport_failures:{count:count(stderr,/ERROR rmcp::transport::worker: worker quit with fatal:/g),availability:stderr===null?"unavailable":"observed"},
  analytics_failures:{count:count(stderr,/WARN codex_analytics::client: failed to send events request:/g),availability:stderr===null?"unavailable":"observed"},
  unknown_agent_signals:{count:count(stderr,/unknown agent|unknown_agent/gi),availability:stderr===null?"unavailable":"observed"},
  fallback_signals:{count:count(stderr,/\bfallback\b/gi),availability:stderr===null?"unavailable":"observed"},
  thread_limit_signals:{count:count(stderr,/thread limit|max(?:imum)? threads|too many threads/gi),availability:stderr===null?"unavailable":"observed"},
  tool_activity:{command_executions:itemCount("command_execution"),tool_calls:itemCount("tool_call"),collaboration_calls:itemCount("collab_tool_call"),availability:stdout===null?"unavailable":"observed"},
};
const sessions=new Map();
for (const record of records) {
  const id=record.thread_id ?? record.payload?.id ?? record.payload?.session_id ?? record.payload?.thread_id ?? record.__fileSession ?? currentThreadId;
  if (!id) continue;
  const session=sessions.get(id) ?? {thread_id:id,session_id:id,model:null,effort:null,usage:null,tool_calls:0,retries:0,rework:null,elapsed_ms:null,quality_outcome:null,timestamps:[]};
  if (record.__timestamp && !Number.isNaN(Date.parse(record.__timestamp))) session.timestamps.push(Date.parse(record.__timestamp));
  if (record.type==="turn_context") { session.model=record.payload?.model ?? null; session.effort=record.payload?.effort ?? null; }
  if (record.type==="session_meta") { session.session_id=record.payload?.id ?? id; session.parent_thread_id=record.payload?.parent_thread_id ?? null; session.agent_role=record.payload?.agent_role ?? null; session.status="started"; }
  if (record.type==="turn.completed") session.usage=record.usage ?? null;
  if (record.type==="event_msg" && record.payload?.type==="token_count") session.usage=record.payload?.info?.total_token_usage ?? session.usage;
  if (record.type==="event_msg" && record.payload?.type==="task_complete") session.status="complete";
  if (record.type==="item.started" && record.item?.type==="tool_call") session.tool_calls += 1;
  if (record.type==="retry") session.retries += 1;
  if (record.type==="item.started" && record.item?.type==="collab_tool_call" && record.item?.tool==="spawn_agent") session.spawn_calls=(session.spawn_calls ?? 0)+1;
  if (record.type==="rework") session.rework=record.count ?? null;
  if (record.type==="quality_outcome") session.quality_outcome=record.outcome ?? null;
  if (record.type==="elapsed") session.elapsed_ms=record.elapsed_ms ?? null;
  sessions.set(id,session);
}
const availability=(field,session)=>session[field]==null?"unavailable":"observed";
const all=[...sessions.values()].map((session)=>{
  if (session.elapsed_ms==null && session.timestamps.length > 1) session.elapsed_ms=Math.max(...session.timestamps)-Math.min(...session.timestamps);
  const {timestamps,...rest}=session;
  return {...rest,availability:{model:availability("model",session),effort:availability("effort",session),usage:availability("usage",session),rework:availability("rework",session),elapsed_ms:availability("elapsed_ms",session),quality_outcome:availability("quality_outcome",session)}};
});
const parent=currentThreadId ?? all.find((session)=>!session.parent_thread_id)?.session_id;
const allowed=new Set([parent,...all.filter((session)=>session.parent_thread_id===parent).map((session)=>session.session_id)]);
const summary={schema_version:2,currency:{availability:"unavailable",reason:"No billable evidence is accepted by this pilot schema."},operational_events,sessions:all.filter((session)=>allowed.has(session.session_id))};
await writeFile(output,`${JSON.stringify(summary,null,2)}\n`);
console.log(`extracted ${summary.sessions.length} telemetry session(s)`);
