#!/usr/bin/env node
import { mkdir, readFile, writeFile, readdir, rename, rm, stat } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { spawnSync } from "node:child_process";
import { mkdtemp } from "node:fs/promises";
import { createInterface } from "node:readline";
import os from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const pilot = JSON.parse(await readFile(path.join(root, "fixtures/routing-pilot-v1/pilot.json"), "utf8"));
const dryRun = process.argv.includes("--dry-run");
const preflight = process.argv.includes("--preflight");
const regenerate = process.argv.includes("--regenerate");
const option = (flag) => { const index = process.argv.indexOf(flag); return index < 0 ? null : process.argv[index + 1]; };
const taskFilter = option("--task");
const armFilter = option("--arm");
const reportRoot = process.env.ROUTING_PILOT_REPORT_ROOT ?? await mkdtemp(path.join(os.tmpdir(), "routing-pilot-report-"));
const codex = ["npx", "-y", "@openai/codex@0.145.0", "exec", "--json"];
const sessionsRoot = process.env.ROUTING_PILOT_CODEX_SESSIONS_ROOT ?? path.join(os.homedir(), ".codex/sessions");
const digest=(value)=>createHash("sha256").update(JSON.stringify(value)).digest("hex");
const permissions={sandbox:"workspace-write",approval_policy:"never",trust_level:"trusted"};
const atomicWrite=async (file,value) => { const temporary=`${file}.${process.pid}.tmp`; await writeFile(temporary,value); await rename(temporary,file); };
async function files(rootDir) {
  const out=[];
  for (const entry of await readdir(rootDir,{withFileTypes:true})) {
    const target=path.join(rootDir,entry.name);
    if(entry.isDirectory()) out.push(...await files(target));
    else if(target.endsWith(".jsonl")) out.push(target);
  }
  return out;
}
async function meta(file) {
  const stream=createReadStream(file,{encoding:"utf8"});
  const lines=createInterface({input:stream,crlfDelay:Infinity});
  try {
    for await (const line of lines) {
      if (!line) continue;
      try {
        const record=JSON.parse(line);
        if (record.type === "session_meta") return record.payload;
      } catch { return null; }
    }
    return null;
  } finally { lines.close(); stream.destroy(); }
}
async function runDate(stdout, stdoutPath) {
  for (const line of stdout.split("\n")) {
    try {
      const timestamp=JSON.parse(line).timestamp;
      if (/^\d{4}-\d{2}-\d{2}/.test(timestamp ?? "")) return timestamp.slice(0,10);
    } catch { /* Ignore malformed non-protocol output. */ }
  }
  return stat(stdoutPath).then((info)=>info.mtime.toISOString().slice(0,10)).catch(()=>null);
}
function validTelemetry(report, arm, parent) {
  if (report.sessions.length !== arm.children.length + 1) return false;
  const parentSession=report.sessions.find((session)=>session.session_id===parent);
  if (!parentSession || parentSession.model !== arm.parent.model || parentSession.effort !== arm.parent.effort || !parentSession.usage) return false;
  return arm.children.every((child)=>report.sessions.some((session)=>session.parent_thread_id===parent && session.agent_role===`pilot_${child.role}` && session.model===child.model && session.effort===child.effort && session.usage));
}
async function telemetry(runDir, arm) {
  const stdout=await readFile(path.join(runDir,"stdout.jsonl"),"utf8");
  const parent=stdout.split("\n").flatMap((line)=>{ try { return [JSON.parse(line)]; } catch { return []; } }).find((record)=>record.type==="thread.started")?.thread_id;
  const date=await runDate(stdout,path.join(runDir,"stdout.jsonl"));
  if(!parent || !date) return null;
  const dateRoot=path.join(sessionsRoot,...date.split("-"));
  const candidates=await files(dateRoot).catch(()=>[]);
  const matches=[];
  for(const file of candidates) {
    const entry=await meta(file);
    if(entry?.id===parent||entry?.parent_thread_id===parent) matches.push({file,entry});
  }
  const ordered=matches.sort((a,b)=>Number(b.entry.id===parent)-Number(a.entry.id===parent)).map((match)=>match.file);
  if(!ordered.length)return null;
  const output=path.join(runDir,"telemetry.json");
  const result=spawnSync(process.execPath,["scripts/extract-routing-pilot-telemetry.mjs","--thread-id",parent,"--input",ordered[0],...ordered.slice(1).flatMap((file)=>["--session",file]),"--stdout",path.join(runDir,"stdout.jsonl"),"--stderr",path.join(runDir,"stderr.txt"),"--output",output],{cwd:root,encoding:"utf8"});
  if(result.status!==0)return null;
  const report=JSON.parse(await readFile(output,"utf8"));
  return validTelemetry(report,arm,parent)&&{path:output,operational_events:report.operational_events};
}

function agentsFor(arm) {
  if (arm.id === "single-sol-medium") return [];
  const children = arm.children.map((child) => ({ ...child, agent_type: `pilot_${child.role}` }));
  return children;
}
function promptFor(task, arm) {
  const base = `${task.prompt}\n\nRead pilot-contract.json and satisfy its oracle exactly. Do not change unrelated files.`;
  if (arm.id === "single-sol-medium") return base;
  const calls = agentsFor(arm).map(({ role, agent_type }) => `Spawn ${agent_type} for the ${role} pass with an isolated task; wait for completion before finalizing.`).join("\n");
  return `${base}\n\n${calls}\nDo not use fallback agent types. Record any spawn failure in your final response.`;
}
function invocation(task,arm,workspace) {
  const prompt=promptFor(task,arm), trust=`projects.${JSON.stringify(workspace)}.trust_level="trusted"`;
  const argv=[...codex,"-C",workspace,"-s",permissions.sandbox,"-c",`approval_policy=\"${permissions.approval_policy}\"`,"-c",trust,"-c","model=\"gpt-5.6-sol\"","-c","model_reasoning_effort=\"medium\"",prompt];
  const contract={task:{id:task.id,prompt:task.prompt,seed_files:task.seed_files,oracle:task.oracle},arm:{id:arm.id,parent:arm.parent,children:arm.children},codex,permissions};
  return {argv,receipt:{contract_fingerprint:digest(contract),prompt_sha256:createHash("sha256").update(prompt).digest("hex"),permissions,start_state_fingerprint:digest(task.seed_files)}};
}
async function prepare(task, arm, runDir) {
  const workspace = path.join(runDir, "workspace"); await mkdir(workspace, { recursive:true });
  for (const [file, content] of Object.entries(task.seed_files)) { const target = path.join(workspace, file); await mkdir(path.dirname(target), {recursive:true}); await writeFile(target, content); }
  const agents = agentsFor(arm); await mkdir(path.join(workspace, ".codex/agents"), {recursive:true});
  const registrations = agents.map(({agent_type}) => `[agents.${agent_type}]\nconfig_file = "./agents/${agent_type}.toml"\n`).join("\n");
  await writeFile(path.join(workspace, ".codex/config.toml"), `${registrations}\n[features.multi_agent_v2]\nenabled = true\nhide_spawn_agent_metadata = true\n`);
  for (const agent of agents) await writeFile(path.join(workspace, `.codex/agents/${agent.agent_type}.toml`), `name = "${agent.agent_type}"\ndescription = "Isolated ${agent.role} pilot agent."\nmodel = "${agent.model}"\nmodel_reasoning_effort = "${agent.effort}"\ndeveloper_instructions = "Perform only your assigned pilot pass, report files changed and any blocker, then stop."\n`);
  await writeFile(path.join(workspace, "pilot-contract.json"), JSON.stringify({task, arm, oracle:task.oracle}, null, 2));
  const git = spawnSync("git", ["init", "-q", workspace], { encoding:"utf8" }); if (git.status !== 0) throw new Error(`git init failed: ${git.stderr}`);
  return workspace;
}
async function oracle(workspace, spec) { try { return { passed:(await readFile(path.join(workspace,spec.path),"utf8")) === spec.equals }; } catch { return { passed:false }; } }
const same=(left,right)=>JSON.stringify(left)===JSON.stringify(right);
async function legacyEvidence(runDir, prior, task, arm, planned, stdout, fatalProcessFailure) {
  if (prior?.receipt || prior?.exit_status !== 0 || prior.workspace !== path.join(runDir,"workspace") || !same(prior.argv,planned.argv) || fatalProcessFailure || !stdout.includes("\"thread.started\"") || !stdout.includes("\"turn.completed\"")) return null;
  const workspaceContract=await readFile(path.join(prior.workspace,"pilot-contract.json"),"utf8").then(JSON.parse).catch(()=>null);
  if (!same(workspaceContract,{task,arm,oracle:task.oracle}) || !(await oracle(prior.workspace,task.oracle)).passed) return null;
  return telemetry(runDir,arm);
}
for (const task of pilot.tasks) if (task.oracle.kind !== "produced_file" && task.seed_files[task.oracle.path] === task.oracle.equals) throw new Error(`${task.id} seed already satisfies its oracle`);
const runs=[];
for (const task of pilot.tasks) for (const arm of pilot.arms) {
  if (taskFilter && task.id !== taskFilter) continue; if (armFilter && arm.id !== armFilter) continue;
  const id=`${task.id}--${arm.id}`, runDir=path.join(reportRoot,id); await mkdir(runDir,{recursive:true});
  const prior = await readFile(path.join(runDir,"run.json"),"utf8").then(JSON.parse).catch(()=>null);
  const existingWorkspace=path.join(runDir,"workspace"), planned=invocation(task,arm,existingWorkspace);
  const receiptMatches=prior?.receipt?.contract_fingerprint===planned.receipt.contract_fingerprint && prior.receipt?.start_state_fingerprint===planned.receipt.start_state_fingerprint;
  const existingStdout=await readFile(path.join(runDir,"stdout.jsonl"),"utf8").catch(()=>"");
  const existingStderr=await readFile(path.join(runDir,"stderr.txt"),"utf8").catch(()=>"");
  const fatalProcessFailure=/(fatal process|process failed|uncaught|\bpanic\b)/i.test(existingStderr);
  if (regenerate) {
    const extracted=await telemetry(runDir,arm);
    const refreshed={...prior,id,task:task.id,arm:arm.id,telemetry:extracted?.path ?? null,operational_events:extracted?.operational_events ?? null,telemetry_regenerated:true};
    if (prior) await writeFile(path.join(runDir,"run.json"),JSON.stringify(refreshed,null,2)+"\n");
    runs.push(refreshed); continue;
  }
  if (!receiptMatches && prior) {
    const extracted=await legacyEvidence(runDir,prior,task,arm,planned,existingStdout,fatalProcessFailure);
    if (extracted) {
      const migrated={...prior,schema_version:2,argv:planned.argv,receipt:planned.receipt,telemetry:extracted.path,operational_events:extracted.operational_events,legacy_evidence_migrated:true};
      await writeFile(path.join(runDir,"run.json"),JSON.stringify(migrated,null,2)+"\n"); runs.push({...migrated,skipped_success:true}); continue;
    }
  }
  if(receiptMatches&&(!prior||prior.exit_status!==0)&&existingStdout.includes("\"thread.started\"")&&existingStdout.includes("\"turn.completed\"")&&!fatalProcessFailure&&(await oracle(existingWorkspace,task.oracle)).passed) { const extracted=await telemetry(runDir,arm); const recovered={schema_version:2,id,task:task.id,arm:arm.id,workspace:existingWorkspace,argv:planned.argv,receipt:planned.receipt,exit_status:0,oracle:{passed:true},telemetry:extracted?.path,operational_events:extracted?.operational_events,recovered_success:true}; if(recovered.telemetry){await writeFile(path.join(runDir,"run.json"),JSON.stringify(recovered,null,2)+"\n");runs.push(recovered);continue;} }
  if (receiptMatches && prior?.exit_status === 0 && prior?.oracle?.passed && prior.workspace && (await oracle(prior.workspace, task.oracle)).passed) { const extracted=await telemetry(runDir,arm); if(extracted) { runs.push({...prior,telemetry:extracted.path,operational_events:extracted.operational_events,skipped_success:true}); continue; } }
  if (preflight) { const stale={...prior,id,task:task.id,arm:arm.id,stale_contract:true,stale_reason:"missing or mismatched invocation receipt/evidence"}; if(prior) await writeFile(path.join(runDir,"run.json"),JSON.stringify(stale,null,2)+"\n"); runs.push(stale); continue; }
  if (prior && !receiptMatches) { await rm(existingWorkspace,{recursive:true,force:true}); await Promise.all(["stdout.jsonl","stderr.txt","telemetry.json"].map((file)=>rm(path.join(runDir,file),{force:true}))); }
  const workspace=await prepare(task,arm,runDir);
  const current=invocation(task,arm,workspace);
  const record={schema_version:2,id,task:task.id,arm:arm.id,workspace,argv:current.argv,receipt:current.receipt,children:agentsFor(arm),dry_run:dryRun,invalidated_prior:Boolean(prior&&!receiptMatches)};
  if (!dryRun) { const result=spawnSync(current.argv[0],current.argv.slice(1),{encoding:"utf8",timeout:900000}); await writeFile(path.join(runDir,"stdout.jsonl"),result.stdout??""); await writeFile(path.join(runDir,"stderr.txt"),result.stderr??""); record.exit_status=result.status; record.process_error=result.error?.message??null; record.oracle=await oracle(workspace,task.oracle); const extracted=await telemetry(runDir,arm); if(extracted) { record.telemetry=extracted.path; record.operational_events=extracted.operational_events; } }
  await writeFile(path.join(runDir,"run.json"),JSON.stringify(record,null,2)+"\n"); runs.push(record);
}
const indexPath=path.join(reportRoot,"index.json");
const allIds=new Set(pilot.tasks.flatMap((task)=>pilot.arms.map((arm)=>`${task.id}--${arm.id}`)));
const selectedIds=new Set(runs.map((run)=>run.id));
const existingIndex=await readFile(indexPath,"utf8").then(JSON.parse).catch(()=>null);
const preserved=taskFilter || armFilter ? (existingIndex?.runs ?? []).filter((run)=>allIds.has(run.id) && !selectedIds.has(run.id)) : [];
const merged=[...preserved,...runs].sort((left,right)=>left.id.localeCompare(right.id));
await atomicWrite(indexPath,JSON.stringify({schema_version:2,report_root:reportRoot,dry_run:dryRun,preflight,regenerate,runs:merged},null,2)+"\n");
console.log(JSON.stringify({report_root:reportRoot,runs:runs.length,index_runs:merged.length,dry_run:dryRun,preflight,regenerate}));
