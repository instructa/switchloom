import { useState, type DragEvent } from "react";
import { buildWorkflowPrompt, defaultOptions, modelLabel, moveCapability, reasoningEfforts, toggleModel, workflowError, type Workflow, type WorkflowOptions } from "@/lib/generator";
import "@/styles/board.css";

type BoardProps = { workflow: Workflow; options: WorkflowOptions; move: (capability: string, owner: string | null) => void };

function CapabilityZone({ workflow, options, owner, move }: BoardProps & { owner: string | null }) {
  const [over, setOver] = useState(false);
  function drop(event: DragEvent) {
    event.preventDefault();
    setOver(false);
    move(event.dataTransfer.getData("application/x-switchloom-capability"), owner);
  }
  return <div className={`${owner ? "dropzone" : "disabled-zone"}${over ? " drag-over" : ""}`} onDragOver={event => { event.preventDefault(); setOver(true); }} onDragLeave={() => setOver(false)} onDrop={drop}>
    {workflow.capabilities.filter(cap => options.owners[cap.id] === owner).map(cap => <div key={cap.id} className="capability" draggable onDragStart={event => { event.dataTransfer.effectAllowed = "move"; event.dataTransfer.setData("application/x-switchloom-capability", cap.id); }}>
      <span className="grip" aria-hidden="true">⠿</span><span>{cap.label}</span>
      <select className="move" aria-label={`Move ${cap.label}`} value="" onChange={event => move(cap.id, event.target.value || null)}>
        <option value="" disabled>↗</option>
        {workflow.slots.filter(slot => options.models[slot.id].enabled).map(slot => <option key={slot.id} value={slot.id}>{slot.id} · {modelLabel(workflow, options.models[slot.id].model)}</option>)}
        <option value="disabled">Disabled</option>
      </select>
    </div>)}
  </div>;
}

function CopyButton({ text, disabled }: { text: string; disabled: boolean }) {
  const [copiedText, setCopiedText] = useState("");
  const [error, setError] = useState(false);
  async function copy() {
    try { await navigator.clipboard.writeText(text); setCopiedText(text); setError(false); }
    catch { setError(true); }
  }
  return <>
    <div className="export"><button type="button" onClick={copy} disabled={disabled}>Copy workflow prompt <span aria-hidden="true">↗</span></button></div>
    <p role="status" className="copy-status">{error ? "Could not copy. Copy the prompt below manually." : text && copiedText === text ? "Copied" : ""}</p>
  </>;
}

export default function Generator({ workflow, options: supplied, onChange, compact = false }: {
  workflow: Workflow; options: WorkflowOptions; onChange: React.Dispatch<React.SetStateAction<WorkflowOptions>>; compact?: boolean;
}) {
  const options = supplied;
  const setOptions = onChange;
  const error = workflowError(workflow, options);
  const prompt = error ? "" : buildWorkflowPrompt(workflow, options);
  function move(capability: string, owner: string | null) {
    setOptions(current => moveCapability(workflow, current, capability, owner === "disabled" ? null : owner));
  }
  function settings(id: string, key: "model" | "effort", value: string) {
    setOptions(current => ({ ...current, models: { ...current.models, [id]: { ...current.models[id], [key]: value } } }));
  }
  return <div className={compact ? "compact-board" : "shell"}>
      {!compact && <div className="intro"><h1>Your workflow.</h1><p className="lede">Assign capabilities to persistent Codex threads. </p></div>}
      <div className="board-heading"><h2>Capabilities</h2><button className="reset" type="button" aria-label="Reset to default" title="Reset to default" onClick={() => setOptions(defaultOptions(workflow))}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M3 10a9 9 0 1 1 2.6 8.4M3 4v6h6" /></svg>
      </button></div>
      <section className="board" aria-label="Model capabilities">
        {workflow.slots.map(slot => {
          const selected = options.models[slot.id];
          const label = modelLabel(workflow, selected.model);
          const efforts = reasoningEfforts(workflow, selected.model);
          const listed = workflow.models.some(model => model.id === selected.model);
          const count = workflow.capabilities.filter(cap => options.owners[cap.id] === slot.id).length;
          return <article key={slot.id} aria-label={`${label} capabilities`} className={`model-column${selected.enabled ? "" : " is-disabled"}`}>
            <header className="model-header">
              <div className="model-title">
                <input type="checkbox" checked={selected.enabled} aria-label={`Enable ${label}`} onChange={event => setOptions(current => toggleModel(workflow, current, slot.id, event.target.checked))} />
                <select className="model-input" aria-label={`${slot.id} model`} value={listed ? selected.model : "custom"} disabled={!selected.enabled} onChange={event => settings(slot.id, "model", event.target.value === "custom" ? "" : event.target.value)}>
                  {workflow.models.map(model => <option key={model.id} value={model.id}>{model.label}</option>)}
                  <option value="custom">Custom GPT model…</option>
                </select>
              </div>
              {!listed && <input className="custom-model" aria-label={`${slot.id} model ID`} placeholder="gpt-…" value={selected.model} disabled={!selected.enabled} spellCheck={false} autoComplete="off" onChange={event => settings(slot.id, "model", event.target.value)} />}
              <div className="model-meta"><span className="slot-id">{slot.id}</span><select aria-label={`${slot.id} effort`} disabled={!selected.enabled} value={selected.effort} onChange={event => settings(slot.id, "effort", event.target.value)}>
                {!efforts.includes(selected.effort) && <option value={selected.effort} disabled>{selected.effort} — choose another</option>}
                {efforts.map(effort => <option key={effort} value={effort}>{effort}</option>)}
              </select><span className="muted">{count} {count === 1 ? "capability" : "capabilities"}</span></div>
            </header>
            {selected.enabled ? <CapabilityZone workflow={workflow} options={options} owner={slot.id} move={move} /> : <p className="model-off">Disabled</p>}
          </article>;
        })}
      </section>
      <section className="disabled-section" aria-label="Disabled capabilities">
        <div className="disabled-heading"><h2>Disabled <span className="disabled-count">{Object.values(options.owners).filter(owner => owner === null).length}</span></h2><p className="muted">Omitted from the prompt and from Jev.</p></div>
        <CapabilityZone workflow={workflow} options={options} owner={null} move={move} />
      </section>
      {error && <p role="alert" className="board-error">{error}</p>}
      {compact && <section className="routing-foot" aria-label="Routing mode">
        <a className="jev-link" href="https://docs.typesafe.ai/introduction" target="_blank" rel="noreferrer">
          <img src="/brand/jev.svg" alt="" width="21" height="32" />
          Jev
        </a>
        <p>{options.jev ? "Jev picks the next capability, not the model. The task text goes to TypeSafe. The CLI needs TYPESAFE_API_KEY. A suggestion is not a send." : "Vanilla uses the same handoff. You set the capability. No TypeSafe call and no API key."}</p>
        <label className="toggle-label"><input type="checkbox" role="switch" checked={options.jev} aria-label="Jev routing" onChange={event => setOptions(current => ({ ...current, jev: event.target.checked }))} /><span className="switch" aria-hidden="true" /></label>
      </section>}
      {!compact && <CopyButton text={prompt} disabled={Boolean(error)} />}
      {!compact && !error && <details className="prompt-section"><summary>Prompt</summary><textarea aria-label="Workflow prompt" readOnly value={prompt} /></details>}
  </div>;
}
