import { useMemo, useState } from "react";
import {
  CheckIcon,
  CopyIcon,
  DownloadSimpleIcon,
  GithubLogoIcon,
  RobotIcon,
  ShieldCheckIcon,
  XLogoIcon,
} from "@phosphor-icons/react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from "@/components/ui/combobox";
import { Field, FieldDescription, FieldGroup, FieldSet, FieldLegend, FieldTitle } from "@/components/ui/field";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { cn } from "@/lib/utils";
import {
  applyPreset,
  changeHost,
  createConfig,
  HOST_IDS,
  HOSTS,
  lifecycleCommands,
  PRESET_IDS,
  PRESETS,
  recipeApplyCommand,
  type HostCatalog,
  type HostId,
  type PresetId,
  ROLE_IDS,
  ROLES,
  type RoleId,
  setEffort,
  setIntegration,
  setModel,
  setRoles,
  setupConfigToml,
  setupSpec,
} from "@/lib/generator";

function HostIcon({ host }: { host: HostId }) {
  return (
    <img
      src={`/brand/${host === "claude-code" ? "claude" : host}.svg`}
      alt=""
      aria-hidden="true"
      className="size-4 shrink-0 object-contain"
    />
  );
}

export default function Generator({ hostCatalog, setupTransport }: { hostCatalog: HostCatalog; setupTransport: { recipePrefix: string; configPath: string } }) {
  const [config, setConfig] = useState(createConfig());
  const [preset, setPreset] = useState<PresetId | "custom">("balanced");
  const [copyState, setCopyState] = useState<"idle" | "copied">("idle");
  const setup = useMemo(() => setupSpec(config, hostCatalog), [config, hostCatalog]);
  const configToml = useMemo(() => setupConfigToml(config, hostCatalog), [config, hostCatalog]);
  const copyCommand = useMemo(() => recipeApplyCommand(config, hostCatalog, setupTransport.recipePrefix), [config, hostCatalog, setupTransport.recipePrefix]);
  const commands = useMemo(() => lifecycleCommands(config, hostCatalog, setupTransport.recipePrefix), [config, hostCatalog, setupTransport.recipePrefix]);
  const host = HOSTS[config.host];

  function downloadConfig() {
    const blob = new Blob([configToml], { type: "application/toml" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = "config.toml";
    link.click();
    URL.revokeObjectURL(url);
  }

  async function copy() {
    await navigator.clipboard.writeText(copyCommand);
    setCopyState("copied");
    window.setTimeout(() => setCopyState("idle"), 1400);
  }

  function selectHost(hostId: HostId) {
    setConfig(changeHost(config, hostId));
    setPreset("balanced");
  }

  function selectPreset(value: PresetId) {
    setConfig(applyPreset(config, value, hostCatalog));
    setPreset(value);
  }

  return (
    <TooltipProvider>
      <div className="min-h-svh">
        <header className="border-b">
          <div className="mx-auto flex h-14 max-w-7xl items-center justify-between px-4 sm:px-6">
            <a href="/" className="flex items-center gap-2 font-heading text-sm font-medium">
              <span className="flex size-7 items-center justify-center bg-primary text-primary-foreground">
                <RobotIcon aria-hidden="true" />
              </span>
              Switchloom
            </a>
            <a
              className={cn(buttonVariants({ variant: "ghost", size: "sm" }))}
              href="https://github.com/instructa/switchloom"
              rel="noreferrer"
            >
              <GithubLogoIcon data-icon="inline-start" aria-hidden="true" />
              GitHub
            </a>
          </div>
        </header>

        <main className="mx-auto flex max-w-7xl flex-col gap-10 px-4 py-10 sm:px-6 sm:py-14">
          <section className="mx-auto flex w-full min-w-0 max-w-3xl flex-col items-center gap-4 text-center">
            <p className="max-w-full text-xs text-muted-foreground">
              Deterministic model routing for coding agents
            </p>
            <h1 className="text-balance font-heading text-3xl font-medium tracking-tight sm:text-5xl">
              Build your coding-agent team.
            </h1>
            <p className="max-w-2xl text-pretty text-sm leading-6 text-muted-foreground sm:text-base">
              Pick a runtime, choose up to four clear roles, then copy a CLI recipe or download the setup config. Codex V2 threads, native subagents, app tasks, and external runners stay separate.
            </p>
          </section>

          <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1.2fr)_minmax(22rem,0.8fr)]">
            <Card className="min-w-0">
              <CardHeader>
                <CardTitle>Configure your team</CardTitle>
                <CardDescription>Choose the host, setup mode, and roles. Fine-tune models only if you want to.</CardDescription>
                <CardAction><Badge variant="secondary">1–4 roles</Badge></CardAction>
              </CardHeader>
              <CardContent>
                <FieldGroup>
                  <FieldSet>
                    <FieldLegend>1. Which runtime are you using?</FieldLegend>
                    <FieldDescription>The generated files match that host's project conventions and evidence boundary.</FieldDescription>
                    <ToggleGroup
                      aria-label="AI agent"
                      value={[config.host]}
                      onValueChange={(values) => values[0] && selectHost(values[0] as HostId)}
                      variant="outline"
                      spacing={0}
                      className="grid w-full grid-cols-1 sm:grid-cols-5"
                    >
                      {HOST_IDS.map((id) => (
                        <ToggleGroupItem key={id} value={id} className="w-full gap-2">
                          <HostIcon host={id} />
                          {HOSTS[id].label}
                        </ToggleGroupItem>
                      ))}
                    </ToggleGroup>
                    <Alert>
                      <ShieldCheckIcon aria-hidden="true" />
                      <AlertTitle>{host.label} output</AlertTitle>
                      <AlertDescription>
                        {host.runtime}: {host.note}
                      </AlertDescription>
                    </Alert>
                  </FieldSet>

                  <Separator />

                  <FieldSet>
                    <FieldLegend>2. Standalone or With Planr?</FieldLegend>
                    <FieldDescription>
                      Standalone writes host-native agent files and {setupTransport.configPath}. With Planr also adds Planr policy files for repositories that already use Planr.
                    </FieldDescription>
                    <ToggleGroup
                      aria-label="Setup mode"
                      value={[config.integration]}
                      onValueChange={(values) => values[0] && setConfig(setIntegration(config, values[0] as "standalone" | "planr"))}
                      variant="outline"
                      spacing={0}
                      className="grid w-full grid-cols-2"
                    >
                      <ToggleGroupItem value="standalone" className="w-full">Standalone</ToggleGroupItem>
                      <ToggleGroupItem value="planr" className="w-full">With Planr</ToggleGroupItem>
                    </ToggleGroup>
                    <Alert>
                      <ShieldCheckIcon aria-hidden="true" />
                      <AlertTitle>{config.integration === "planr" ? "Planr integration" : "Standalone setup"}</AlertTitle>
                      <AlertDescription>
                        {config.integration === "planr"
                          ? "Use this only when the target repository has Planr. Switchloom remains the CLI transport; Planr consumes semantic roles, agent_type, fork_turns, and policy files without owning model/provider catalogs."
                          : "No Planr dependency is required. The CLI expands the setup into repository-local files for the selected host."}
                      </AlertDescription>
                    </Alert>
                  </FieldSet>

                  <Separator />

                  <FieldSet>
                    <FieldLegend>3. Which roles do you need?</FieldLegend>
                    <FieldDescription>Orchestrator is always included. Add only roles that create a real handoff.</FieldDescription>
                    <ToggleGroup
                      aria-label="Team roles"
                      multiple
                      value={config.roles}
                      onValueChange={(values) => setConfig(setRoles(config, values))}
                      variant="outline"
                      spacing={0}
                      className="grid w-full grid-cols-2 sm:grid-cols-4"
                    >
                      {ROLE_IDS.map((role) => (
                        <ToggleGroupItem key={role} value={role} disabled={role === "orchestrator"} className="w-full">
                          {ROLES[role].label}
                        </ToggleGroupItem>
                      ))}
                    </ToggleGroup>
                  </FieldSet>

                  <Separator />

                  <FieldSet>
                    <FieldLegend>4. Tune each role</FieldLegend>
                    <FieldDescription>Start with a team-wide preset, then override any role below.</FieldDescription>
                    <FieldGroup>
                      <Field>
                        <div className="flex items-center justify-between gap-3">
                          <FieldTitle>Team preset</FieldTitle>
                          {preset === "custom" && <Badge variant="secondary">Custom</Badge>}
                        </div>
                        <ToggleGroup
                          aria-label="Team preset"
                          value={preset === "custom" ? [] : [preset]}
                          onValueChange={(values) => values[0] && selectPreset(values[0] as PresetId)}
                          variant="outline"
                          spacing={0}
                          className="grid w-full grid-cols-3"
                        >
                          {PRESET_IDS.map((id) => (
                            <ToggleGroupItem key={id} value={id} className="w-full">
                              {PRESETS[id].label}
                            </ToggleGroupItem>
                          ))}
                        </ToggleGroup>
                        <FieldDescription>
                          {preset === "custom" ? "Per-role choices override the preset." : PRESETS[preset].short}
                        </FieldDescription>
                      </Field>
                      {config.roles.map((role) => (
                        <RoleCard
                          key={`${config.host}-${role}`}
                          role={role}
                          config={config}
                          hostCatalog={hostCatalog}
                          onModel={(model) => {
                            setConfig(setModel(config, role, model, hostCatalog));
                            setPreset("custom");
                          }}
                          onEffort={(effort) => {
                            setConfig(setEffort(config, role, effort, hostCatalog));
                            setPreset("custom");
                          }}
                        />
                      ))}
                    </FieldGroup>
                  </FieldSet>
                </FieldGroup>
              </CardContent>
            </Card>

            <Card className="min-w-0 lg:sticky lg:top-6">
              <CardHeader>
                <CardTitle>Your {host.label} team</CardTitle>
                <CardDescription>{config.roles.length} focused roles · {config.integration === "planr" ? "Planr setup spec" : "standalone setup spec"}</CardDescription>
                <CardAction><Badge>{config.roles.length}/4</Badge></CardAction>
              </CardHeader>
              <CardContent>
                <Tabs defaultValue="team">
                  <TabsList className="w-full">
                    <TabsTrigger value="team">Team</TabsTrigger>
                    <TabsTrigger value="commands">Commands</TabsTrigger>
                    <TabsTrigger value="spec">Spec</TabsTrigger>
                  </TabsList>
                  <TabsContent value="team" className="pt-3">
                    <div className="flex flex-col gap-3">
                      {config.roles.map((role, index) => {
                        const assignment = config.assignments[role];
                        return (
                          <div key={role} className="grid grid-cols-[1.5rem_1fr] gap-3">
                            <span className="flex size-6 items-center justify-center bg-muted text-[0.7rem]">{index + 1}</span>
                            <div className="min-w-0">
                              <p className="font-medium">{ROLES[role].label}</p>
                              <p className="truncate text-muted-foreground">
                                {assignment.model}{assignment.effort ? ` · ${assignment.effort}` : ""}
                              </p>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </TabsContent>
                  <TabsContent value="commands" className="pt-3">
                    <ol className="flex flex-col gap-2">
                      {commands.map((command) => (
                        <li key={command} className="rounded-sm bg-muted px-2 py-1 font-mono text-[0.68rem] leading-5 text-muted-foreground">
                          {command}
                        </li>
                      ))}
                    </ol>
                  </TabsContent>
                  <TabsContent value="spec" className="pt-3">
                    <pre className="max-h-80 overflow-auto rounded-sm bg-muted p-3 text-[0.68rem] leading-5 text-muted-foreground">
                      {JSON.stringify(setup, null, 2)}
                    </pre>
                  </TabsContent>
                </Tabs>
              </CardContent>
              <CardFooter className="flex-col items-stretch gap-2">
                <Button size="lg" onClick={copy}>
                  {copyState === "copied" ? <CheckIcon data-icon="inline-start" aria-hidden="true" /> : <CopyIcon data-icon="inline-start" aria-hidden="true" />}
                  {copyState === "copied" ? "Copied command" : "Copy npx recipe command"}
                </Button>
                <Button variant="outline" onClick={downloadConfig}>
                  <DownloadSimpleIcon data-icon="inline-start" aria-hidden="true" />
                  Download .switchloom/config.toml
                </Button>
                <p className="pt-1 text-center text-[0.7rem] leading-5 text-muted-foreground">
                  Preview before apply, run doctor to check the host, and review every repository-local change before confirming setup.
                </p>
              </CardFooter>
            </Card>
          </div>
        </main>

        <footer className="border-t">
          <div className="mx-auto flex max-w-7xl flex-col items-center justify-between gap-2 px-4 py-5 text-xs text-muted-foreground sm:flex-row sm:px-6">
            <p>
              Made by{" "}
              <a className="font-medium text-foreground underline-offset-4 hover:underline" href="https://kevinkern.dev" rel="noreferrer" target="_blank">
                kevinkern
              </a>
            </p>
            <a className="flex items-center gap-1.5 text-foreground underline-offset-4 hover:underline" href="https://x.com/kevinkern" rel="noreferrer" target="_blank">
              <XLogoIcon aria-hidden="true" className="size-4" />
              @kevinkern
            </a>
          </div>
        </footer>
      </div>
    </TooltipProvider>
  );
}

function RoleCard({ role, config, hostCatalog, onModel, onEffort }: {
  role: RoleId;
  config: ReturnType<typeof createConfig>;
  hostCatalog: HostCatalog;
  onModel: (model: string) => void;
  onEffort: (effort: string) => void;
}) {
  const host = HOSTS[config.host];
  const models = hostCatalog[config.host].models;
  const assignment = config.assignments[role];
  const model = models.find((candidate) => candidate.id === assignment.model)!;
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{ROLES[role].label}</CardTitle>
        <CardDescription>{ROLES[role].short}</CardDescription>
        <CardAction><Badge variant={model.tier === "standard" ? "secondary" : "outline"}>{model.tier}</Badge></CardAction>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <Field>
            <FieldTitle>Model</FieldTitle>
            {config.host === "cursor" ? (
              <Combobox
                items={models}
                value={model}
                onValueChange={(value) => value && onModel(value.id)}
                itemToStringValue={(option) => option.label}
                autoHighlight
              >
                <ComboboxInput
                  aria-label={`${ROLES[role].label} model`}
                  placeholder="Search latest models…"
                  className="w-full"
                />
                <ComboboxContent>
                  <ComboboxEmpty>No current model found.</ComboboxEmpty>
                  <ComboboxList>
                    {(option) => (
                      <ComboboxItem key={option.id} value={option}>
                        <span className="min-w-0 flex-1 truncate">{option.label}</span>
                        <span className="text-muted-foreground">{option.provider}</span>
                      </ComboboxItem>
                    )}
                  </ComboboxList>
                </ComboboxContent>
              </Combobox>
            ) : (
              <ToggleGroup
                aria-label={`${ROLES[role].label} model`}
                value={[assignment.model]}
                onValueChange={(values) => values[0] && onModel(values[0])}
                variant="outline"
                spacing={1}
                className="flex w-full flex-wrap"
              >
                {models.map((option) => {
                  return option.disabledReason ? (
                    <Tooltip key={option.id}>
                      <TooltipTrigger render={<span className="inline-block w-fit" />}>
                        <ToggleGroupItem value={option.id} disabled>
                          {option.label}
                        </ToggleGroupItem>
                      </TooltipTrigger>
                      <TooltipContent>
                        <p>{option.disabledReason}</p>
                      </TooltipContent>
                    </Tooltip>
                  ) : (
                    <ToggleGroupItem key={option.id} value={option.id}>
                      {option.label}
                    </ToggleGroupItem>
                  );
                })}
              </ToggleGroup>
            )}
          </Field>
          {host.effortLabel && model.efforts.length > 0 && (
            <Field>
              <FieldTitle>{host.effortLabel}</FieldTitle>
              <ToggleGroup
                aria-label={`${ROLES[role].label} ${host.effortLabel}`}
                value={assignment.effort ? [assignment.effort] : []}
                onValueChange={(values) => values[0] && onEffort(values[0])}
                variant="outline"
                spacing={1}
              >
                {model.efforts.map((effort) => (
                  <ToggleGroupItem key={effort} value={effort}>
                    {effort}
                  </ToggleGroupItem>
                ))}
              </ToggleGroup>
              {model.efforts.includes("ultra") && (
                <FieldDescription>
                  Ultra enables automatic multi-agent delegation. It is manual-only and never selected by a preset.
                </FieldDescription>
              )}
            </Field>
          )}
        </FieldGroup>
      </CardContent>
    </Card>
  );
}
