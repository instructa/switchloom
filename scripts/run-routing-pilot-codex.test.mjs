import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
test("pilot harness dry run constructs nine matched root invocations", async (t) => { const out=await mkdtemp(path.join(os.tmpdir(),"pilot-harness-")); t.after(()=>rm(out,{recursive:true,force:true})); const run=spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--dry-run"],{encoding:"utf8",env:{...process.env,ROUTING_PILOT_REPORT_ROOT:out}}); assert.equal(run.status,0,run.stderr); const index=JSON.parse(await readFile(path.join(out,"index.json"))); assert.equal(index.runs.length,9); assert.ok(index.runs.every((r)=>r.argv.some((arg)=>arg.includes("gpt-5.6-sol")) && r.argv.some((arg)=>arg.includes("trust_level")) && r.receipt?.contract_fingerprint && r.receipt?.prompt_sha256)); assert.equal(index.runs.filter((r)=>r.arm==="single-sol-medium")[0].children.length,0); assert.deepEqual(index.runs.find((r)=>r.arm==="routed-sol-terra").children.map((x)=>x.effort),["medium","medium","low"]); const routed=index.runs.find((r)=>r.arm==="routed-sol-terra"); assert.match(await readFile(path.join(routed.workspace,".codex/agents/pilot_implementer.toml"),"utf8"),/description/); assert.equal(await readFile(path.join(index.runs.find((r)=>r.id==="doctor-semantic-role--single-sol-medium").workspace,"role.txt"),"utf8"),"switchloom_reviewer\n"); assert.equal(spawnSync("git",["-C",routed.workspace,"rev-parse","--is-inside-work-tree"],{encoding:"utf8"}).stdout.trim(),"true"); });

test("filtered run replaces only selected index rows", async (t) => {
  const out=await mkdtemp(path.join(os.tmpdir(),"pilot-index-")); t.after(()=>rm(out,{recursive:true,force:true})); const env={...process.env,ROUTING_PILOT_REPORT_ROOT:out};
  assert.equal(spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--dry-run"],{encoding:"utf8",env}).status,0);
  const indexPath=path.join(out,"index.json"), index=JSON.parse(await readFile(indexPath)); const preserved=index.runs.find((run)=>run.id==="native-provenance--routed-sol-terra"); preserved.marker="keep"; await writeFile(indexPath,JSON.stringify(index));
  const filtered=spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--dry-run","--task","doctor-semantic-role","--arm","single-sol-medium"],{encoding:"utf8",env}); assert.equal(filtered.status,0,filtered.stderr);
  const merged=JSON.parse(await readFile(indexPath)); assert.equal(merged.runs.length,9); assert.equal(merged.runs.find((run)=>run.id===preserved.id).marker,"keep"); assert.ok(merged.runs.find((run)=>run.id==="doctor-semantic-role--single-sol-medium"));
});

test("recovery discovers only same-day session metadata and persists a single-agent success", async (t) => {
  const out=await mkdtemp(path.join(os.tmpdir(),"pilot-recovery-"));
  const sessions=await mkdtemp(path.join(os.tmpdir(),"pilot-sessions-"));
  t.after(()=>Promise.all([rm(out,{recursive:true,force:true}),rm(sessions,{recursive:true,force:true})]));
  const id="doctor-semantic-role--single-sol-medium", parent="parent-thread-123";
  const runDir=path.join(out,id), workspace=path.join(runDir,"workspace");
  const env={...process.env,ROUTING_PILOT_REPORT_ROOT:out,ROUTING_PILOT_CODEX_SESSIONS_ROOT:sessions};
  assert.equal(spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--dry-run","--task","doctor-semantic-role","--arm","single-sol-medium"],{encoding:"utf8",env}).status,0);
  await mkdir(workspace,{recursive:true});
  await writeFile(path.join(workspace,"role.txt"),"switchloom_implementer\n");
  await writeFile(path.join(runDir,"stdout.jsonl"),`${JSON.stringify({timestamp:"2026-07-24T12:00:00.000Z",type:"thread.started",thread_id:parent})}\n${JSON.stringify({type:"turn.completed"})}\n`);
  await writeFile(path.join(runDir,"stderr.txt"),"");
  const sessionDir=path.join(sessions,"2026","07","24");
  await mkdir(sessionDir,{recursive:true});
  await writeFile(path.join(sessionDir,`rollout-${parent}.jsonl`),[
    {type:"session_meta",payload:{id:parent,session_id:parent}},
    {type:"turn_context",payload:{model:"gpt-5.6-sol",effort:"medium"}},
    {type:"event_msg",payload:{type:"token_count",info:{total_token_usage:{input_tokens:1,output_tokens:1,total_tokens:2}}}},
    {type:"event_msg",payload:{type:"task_complete"}},
  ].map(JSON.stringify).join("\n")+"\n");
  const irrelevant=path.join(sessions,"2026","07","23");
  await mkdir(irrelevant,{recursive:true});
  await writeFile(path.join(irrelevant,"large-irrelevant.jsonl"),"not-json ".repeat(500_000));
  const run=spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--dry-run","--task","doctor-semantic-role","--arm","single-sol-medium"],{encoding:"utf8",env});
  assert.equal(run.status,0,run.stderr);
  const recovered=JSON.parse(await readFile(path.join(runDir,"run.json")));
  assert.equal(recovered.exit_status,0);
  assert.equal(recovered.recovered_success,true);
  assert.equal(JSON.parse(await readFile(path.join(runDir,"telemetry.json"))).sessions.length,1);
});

test("a stale contract fingerprint invalidates a prior run instead of resuming it", async (t) => {
  const out=await mkdtemp(path.join(os.tmpdir(),"pilot-fingerprint-")); t.after(()=>rm(out,{recursive:true,force:true}));
  const env={...process.env,ROUTING_PILOT_REPORT_ROOT:out};
  const args=["scripts/run-routing-pilot-codex.mjs","--dry-run","--task","doctor-semantic-role","--arm","single-sol-medium"];
  assert.equal(spawnSync(process.execPath,args,{encoding:"utf8",env}).status,0);
  const runPath=path.join(out,"doctor-semantic-role--single-sol-medium","run.json"), prior=JSON.parse(await readFile(runPath));
  await writeFile(runPath,JSON.stringify({...prior,receipt:{...prior.receipt,contract_fingerprint:"stale"}}));
  await writeFile(path.join(prior.workspace,"role.txt"),"tampered\n");
  const rerun=spawnSync(process.execPath,args,{encoding:"utf8",env}); assert.equal(rerun.status,0,rerun.stderr);
  const next=JSON.parse(await readFile(runPath));
  assert.equal(next.invalidated_prior,true); assert.notEqual(next.receipt.contract_fingerprint,"stale");
  assert.equal(await readFile(path.join(next.workspace,"role.txt"),"utf8"),"switchloom_reviewer\n");
});

test("preflight migrates only matching legacy evidence and leaves changed Doctor evidence stale", async (t) => {
  const out=await mkdtemp(path.join(os.tmpdir(),"pilot-legacy-")), sessions=await mkdtemp(path.join(os.tmpdir(),"pilot-legacy-sessions-"));
  t.after(()=>Promise.all([rm(out,{recursive:true,force:true}),rm(sessions,{recursive:true,force:true})]));
  const env={...process.env,ROUTING_PILOT_REPORT_ROOT:out,ROUTING_PILOT_CODEX_SESSIONS_ROOT:sessions};
  const makeLegacy=async (task, oracleValue, oldDoctorSeed=false) => {
    const args=["scripts/run-routing-pilot-codex.mjs","--dry-run","--task",task,"--arm","single-sol-medium"];
    assert.equal(spawnSync(process.execPath,args,{encoding:"utf8",env}).status,0);
    const id=`${task}--single-sol-medium`, runDir=path.join(out,id), runPath=path.join(runDir,"run.json"), prior=JSON.parse(await readFile(runPath));
    await writeFile(path.join(prior.workspace,task==="doctor-semantic-role"?"role.txt":"config.toml"),oracleValue);
    if (oldDoctorSeed) { const contract=JSON.parse(await readFile(path.join(prior.workspace,"pilot-contract.json"))); contract.task.seed_files["role.txt"]="switchloom_implementer\n"; await writeFile(path.join(prior.workspace,"pilot-contract.json"),JSON.stringify(contract,null,2)); }
    const thread=`${task}-parent`; await writeFile(path.join(runDir,"stdout.jsonl"),`${JSON.stringify({timestamp:"2026-07-24T12:00:00.000Z",type:"thread.started",thread_id:thread})}\n${JSON.stringify({type:"turn.completed"})}\n`); await writeFile(path.join(runDir,"stderr.txt"),"");
    const dir=path.join(sessions,"2026","07","24"); await mkdir(dir,{recursive:true}); await writeFile(path.join(dir,`${thread}.jsonl`),[{type:"session_meta",payload:{id:thread}},{type:"turn_context",payload:{model:"gpt-5.6-sol",effort:"medium"}},{type:"event_msg",payload:{type:"token_count",info:{total_token_usage:{input_tokens:1}}}},{type:"event_msg",payload:{type:"task_complete"}}].map(JSON.stringify).join("\n")+"\n");
    await writeFile(runPath,JSON.stringify({...prior,exit_status:0,oracle:{passed:true},receipt:undefined}));
    return {id,runPath};
  };
  const v2=await makeLegacy("v2-flag-repair","enabled = true\nhide_spawn_agent_metadata = true\n"), doctor=await makeLegacy("doctor-semantic-role","switchloom_implementer\n",true);
  const result=spawnSync(process.execPath,["scripts/run-routing-pilot-codex.mjs","--preflight"],{encoding:"utf8",env}); assert.equal(result.status,0,result.stderr);
  const migrated=JSON.parse(await readFile(v2.runPath)), stale=JSON.parse(await readFile(doctor.runPath));
  assert.equal(migrated.legacy_evidence_migrated,true); assert.ok(migrated.receipt.contract_fingerprint); assert.equal(stale.stale_contract,true);
});
