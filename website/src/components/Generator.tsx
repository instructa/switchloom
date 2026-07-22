import { useMemo, useState } from "react";
import {
  ArrowCounterClockwiseIcon,
  CheckIcon,
  CopyIcon,
  GithubLogoIcon,
  QuestionIcon,
  RobotIcon,
  ShieldCheckIcon,
  TrashIcon,
  WarningCircleIcon,
  XLogoIcon,
} from "@phosphor-icons/react";

import { AgentTeamDialog } from "@/components/AgentTeamDialog";
import { EffortStrengthPicker } from "@/components/EffortStrengthPicker";
import { ModelStrengthKugeln } from "@/components/ModelStrengthKugeln";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
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
import { modelFamilyInfo } from "@/lib/model-family";
import {
  applyPreset,
  canRenderParentRecommendation,
  changeHost,
  choosePreset,
  createConfig,
  createPresetSelection,
  HOST_IDS,
  HOSTS,
  isPresetDirty,
  lifecycleCommands,
  markPresetCustom,
  PRESET_IDS,
  PRESETS,
  parentRecommendationEffortCopy,
  primaryRecommendation,
  recipeApplyCommand,
  removeChildRole,
  resetRolesToPreset,
  type ChildRoleId,
  type HostCatalog,
  type HostId,
  type PresetId,
  ROLES,
  type RoleId,
  setEffort,
  setIntegration,
  setModel,
  setupSpec,
  setupSpecRoleIds,
} from "@/lib/generator";
import { providerOnboarding } from "@/lib/onboarding";

function HostIcon({ host }: { host: HostId }) {
  return (
    <img
      src={`/brand/${host === "claude-code" ? "claude" : host}.svg`}
      alt=""
      aria-hidden="true"
      className={cn(
        "size-4 shrink-0 object-contain",
        (host === "cursor" || host === "opencode" || host === "pi") && "group-aria-[pressed=true]/toggle:invert",
      )}
    />
  );
}

function truncateCommandPreview(command: string) {
  return command.replace(/--recipe '[^']+'/, "--recipe 'sw1_…'");
}

type GeneratorProps = {
  hostCatalog: HostCatalog;
  setupTransport: { recipePrefix: string; configPath: string };
};

type GeneratorConfigState = ReturnType<typeof createConfig>;
type PresetSelectionState = ReturnType<typeof createPresetSelection>;
type OnboardingState = ReturnType<typeof providerOnboarding>;

export default function Generator({ hostCatalog, setupTransport }: GeneratorProps) {
  const [config, setConfig] = useState(() => createConfig());
  const [preset, setPreset] = useState(() => createPresetSelection());
  const [copiedCommandId, setCopiedCommandId] = useState<string | null>(null);
  const [resetDialogOpen, setResetDialogOpen] = useState(false);
  const setup = useMemo(() => setupSpec(config, hostCatalog), [config, hostCatalog]);
  const copyCommand = useMemo(() => recipeApplyCommand(config, hostCatalog, setupTransport.recipePrefix), [config, hostCatalog, setupTransport.recipePrefix]);
  const commands = useMemo(() => lifecycleCommands(config, hostCatalog, setupTransport.recipePrefix), [config, hostCatalog, setupTransport.recipePrefix]);
  const onboarding = useMemo(() => providerOnboarding(config.host, copyCommand), [config.host, copyCommand]);
  const host = HOSTS[config.host];
  const rolesDirty = useMemo(() => isPresetDirty(config, preset.lastSelected, hostCatalog), [config, preset.lastSelected, hostCatalog]);
  const hasHostManagedParent = canRenderParentRecommendation(config);
  const childRoleCount = config.roles.length;
  const outputRoleIds = useMemo(() => setupSpecRoleIds(config), [config]);

  async function copyLifecycleCommand(id: string, command: string) {
    try {
      await navigator.clipboard.writeText(command);
      setCopiedCommandId(id);
      window.setTimeout(() => setCopiedCommandId((current) => current === id ? null : current), 1400);
    } catch {
      setCopiedCommandId(null);
    }
  }

  function selectHost(hostId: HostId) {
    const nextConfig = changeHost(config, hostId);
    const nextPreset = choosePreset(preset, "balanced");
    setConfig(nextConfig);
    setPreset(isPresetDirty(nextConfig, "balanced", hostCatalog) ? markPresetCustom(nextPreset) : nextPreset);
  }

  function selectPreset(value: PresetId) {
    setConfig(applyPreset(config, value, hostCatalog));
    setPreset(choosePreset(preset, value));
  }

  function removeRole(role: ChildRoleId) {
    const nextConfig = removeChildRole(config, role);
    if (nextConfig === config) return;
    setConfig(nextConfig);
    setPreset(markPresetCustom(preset));
  }

  function confirmResetRoles() {
    setConfig(resetRolesToPreset(config, preset.lastSelected, hostCatalog));
    setPreset(choosePreset(preset, preset.lastSelected));
    setResetDialogOpen(false);
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
              <Badge className="border-transparent bg-accent text-accent-foreground">Beta</Badge>
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
              Pick a runtime, choose your roles, and get a ready-to-run setup.
            </p>
          </section>

          <div className="grid items-start gap-6 lg:grid-cols-[minmax(0,1.2fr)_minmax(22rem,0.8fr)]">
            <GeneratorConfigPanel
              config={config}
              hasHostManagedParent={hasHostManagedParent}
              host={host}
              hostCatalog={hostCatalog}
              preset={preset}
              resetDialogOpen={resetDialogOpen}
              rolesDirty={rolesDirty}
              onConfirmResetRoles={confirmResetRoles}
              onEffort={(role, effort) => {
                setConfig(setEffort(config, role, effort, hostCatalog));
                setPreset(markPresetCustom(preset));
              }}
              onIntegration={(integration) => setConfig(setIntegration(config, integration))}
              onModel={(role, model) => {
                setConfig(setModel(config, role, model, hostCatalog));
                setPreset(markPresetCustom(preset));
              }}
              onPreset={selectPreset}
              onRemoveRole={removeRole}
              onResetDialogOpenChange={setResetDialogOpen}
              onSelectHost={selectHost}
            />

            <GeneratorSummaryPanel
              childRoleCount={childRoleCount}
              commands={commands}
              config={config}
              copiedCommandId={copiedCommandId}
              hasHostManagedParent={hasHostManagedParent}
              host={host}
              onboarding={onboarding}
              outputRoleIds={outputRoleIds}
              setup={setup}
              onCopyCommand={copyLifecycleCommand}
            />
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

function GeneratorConfigPanel({ config, hasHostManagedParent, host, hostCatalog, preset, resetDialogOpen, rolesDirty, onConfirmResetRoles, onEffort, onIntegration, onModel, onPreset, onRemoveRole, onResetDialogOpenChange, onSelectHost }: {
  config: GeneratorConfigState;
  hasHostManagedParent: boolean;
  host: (typeof HOSTS)[HostId];
  hostCatalog: HostCatalog;
  preset: PresetSelectionState;
  resetDialogOpen: boolean;
  rolesDirty: boolean;
  onConfirmResetRoles: () => void;
  onEffort: (role: ChildRoleId, effort: string) => void;
  onIntegration: (integration: "standalone" | "planr") => void;
  onModel: (role: ChildRoleId, model: string) => void;
  onPreset: (preset: PresetId) => void;
  onRemoveRole: (role: ChildRoleId) => void;
  onResetDialogOpenChange: (open: boolean) => void;
  onSelectHost: (host: HostId) => void;
}) {
  return (
    <Card className="min-w-0">
      <CardHeader>
        <CardTitle>Configure your team</CardTitle>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <RuntimeFieldSet config={config} host={host} onSelectHost={onSelectHost} />
          <Separator />
          <IntegrationFieldSet config={config} onIntegration={onIntegration} />
          <Separator />
          <RoleTuningFieldSet
            config={config}
            hasHostManagedParent={hasHostManagedParent}
            host={host}
            hostCatalog={hostCatalog}
            preset={preset}
            resetDialogOpen={resetDialogOpen}
            rolesDirty={rolesDirty}
            onConfirmResetRoles={onConfirmResetRoles}
            onEffort={onEffort}
            onModel={onModel}
            onPreset={onPreset}
            onRemoveRole={onRemoveRole}
            onResetDialogOpenChange={onResetDialogOpenChange}
          />
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function RuntimeFieldSet({ config, host, onSelectHost }: {
  config: GeneratorConfigState;
  host: (typeof HOSTS)[HostId];
  onSelectHost: (host: HostId) => void;
}) {
  return (
    <FieldSet>
      <div className="mb-2.5 flex items-center gap-1">
        <FieldLegend className="mb-0">1. Which runtime are you using?</FieldLegend>
        <Tooltip>
          <TooltipTrigger
            render={
              <button
                type="button"
                className={cn(
                  buttonVariants({ variant: "ghost", size: "icon-xs" }),
                  "text-muted-foreground",
                )}
                aria-label={`${host.label} output details`}
              />
            }
          >
            <QuestionIcon aria-hidden="true" />
          </TooltipTrigger>
          <TooltipContent side="bottom" align="start" className="max-w-sm flex-col items-start gap-1 py-2 text-left">
            <p className="font-medium">{host.label}: {host.runtime}</p>
            <p className="text-pretty opacity-80">{host.note}</p>
          </TooltipContent>
        </Tooltip>
      </div>
      <FieldDescription>The generated files match that host's project conventions and evidence boundary.</FieldDescription>
      <ToggleGroup
        aria-label="AI agent"
        value={[config.host]}
        onValueChange={(values) => values[0] && onSelectHost(values[0] as HostId)}
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
      {config.host !== "codex" && (
        <Alert className="border-amber-300 bg-amber-50 text-amber-950">
          <WarningCircleIcon aria-hidden="true" />
          <AlertTitle>Experimental configuration</AlertTitle>
          <AlertDescription className="text-amber-900/80">
            The {host.label} configuration is experimental, and its model catalog is not complete yet. Want to help? Open a pull request at{" "}
            <a href="https://github.com/instructa/switchloom" rel="noreferrer" target="_blank">
              instructa/switchloom
            </a>
            .
          </AlertDescription>
        </Alert>
      )}
    </FieldSet>
  );
}

function IntegrationFieldSet({ config, onIntegration }: {
  config: GeneratorConfigState;
  onIntegration: (integration: "standalone" | "planr") => void;
}) {
  return (
    <FieldSet>
      <FieldLegend>2. Standalone or With Planr?</FieldLegend>
      <FieldDescription>
        Standalone sets up host-native agents in your project. With Planr also adds Planr policy files when the repository already uses Planr.
      </FieldDescription>
      <ToggleGroup
        aria-label="Setup mode"
        value={[config.integration]}
        onValueChange={(values) => values[0] && onIntegration(values[0] as "standalone" | "planr")}
        variant="outline"
        spacing={0}
        className="grid w-full grid-cols-2"
      >
        <ToggleGroupItem value="standalone" className="w-full">Standalone</ToggleGroupItem>
        <ToggleGroupItem value="planr" className="w-full gap-2">
          <img
            src="/brand/planr.svg"
            alt=""
            aria-hidden="true"
            className="size-4 shrink-0 object-contain group-aria-[pressed=true]/toggle:invert"
          />
          With Planr
        </ToggleGroupItem>
      </ToggleGroup>
      {config.integration === "planr" && (
        <Alert>
          <ShieldCheckIcon aria-hidden="true" />
          <AlertTitle>Planr integration</AlertTitle>
          <AlertDescription>
            Use it with{" "}
            <a href="https://planr.so" target="_blank" rel="noreferrer">
              planr.so
            </a>
            .
          </AlertDescription>
        </Alert>
      )}
    </FieldSet>
  );
}

function RoleTuningFieldSet({ config, hasHostManagedParent, host, hostCatalog, preset, resetDialogOpen, rolesDirty, onConfirmResetRoles, onEffort, onModel, onPreset, onRemoveRole, onResetDialogOpenChange }: {
  config: GeneratorConfigState;
  hasHostManagedParent: boolean;
  host: (typeof HOSTS)[HostId];
  hostCatalog: HostCatalog;
  preset: PresetSelectionState;
  resetDialogOpen: boolean;
  rolesDirty: boolean;
  onConfirmResetRoles: () => void;
  onEffort: (role: ChildRoleId, effort: string) => void;
  onModel: (role: ChildRoleId, model: string) => void;
  onPreset: (preset: PresetId) => void;
  onRemoveRole: (role: ChildRoleId) => void;
  onResetDialogOpenChange: (open: boolean) => void;
}) {
  return (
    <FieldSet>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <FieldLegend>3. Tune each role</FieldLegend>
          <FieldDescription>Start with a team-wide preset, then override any role below.</FieldDescription>
        </div>
        <ResetRolesDialog
          host={host}
          open={resetDialogOpen}
          preset={preset}
          rolesDirty={rolesDirty}
          onConfirm={onConfirmResetRoles}
          onOpenChange={onResetDialogOpenChange}
        />
      </div>
      <FieldGroup>
        <PresetField preset={preset} onPreset={onPreset} />
        {hasHostManagedParent && (
          <ParentRecommendationCard config={config} preset={preset.lastSelected} />
        )}
        <div className="flex flex-col gap-3">
          {config.roles.map((role, index) => (
            <ConnectedChildCard key={`${config.host}-${role}`} isLast={index === config.roles.length - 1}>
              <RoleCard
                role={role}
                config={config}
                hostCatalog={hostCatalog}
                canRemove={config.roles.length > 1}
                onRemove={() => onRemoveRole(role)}
                onModel={(model) => onModel(role, model)}
                onEffort={(effort) => onEffort(role, effort)}
              />
            </ConnectedChildCard>
          ))}
        </div>
      </FieldGroup>
    </FieldSet>
  );
}

function ResetRolesDialog({ host, open, preset, rolesDirty, onConfirm, onOpenChange }: {
  host: (typeof HOSTS)[HostId];
  open: boolean;
  preset: PresetSelectionState;
  rolesDirty: boolean;
  onConfirm: () => void;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogTrigger render={<Button type="button" size="sm" variant="outline" disabled={!rolesDirty} aria-label="Reset roles" />}>
        <ArrowCounterClockwiseIcon data-icon="inline-start" aria-hidden="true" />
        Reset roles
      </DialogTrigger>
      <DialogContent className="overflow-hidden p-0 sm:max-w-lg" showCloseButton={false}>
        <DialogHeader className="border-b px-5 py-5 sm:px-6">
          <div className="mb-2 flex items-center gap-2">
            <span className="flex size-8 items-center justify-center border bg-destructive/10 text-destructive">
              <WarningCircleIcon aria-hidden="true" />
            </span>
            <Badge variant="secondary">Destructive action</Badge>
          </div>
          <DialogTitle className="text-lg">Reset roles?</DialogTitle>
          <DialogDescription className="max-w-md text-sm leading-6">
            Restore the {PRESETS[preset.lastSelected].label} preset roles, models, effort, and usage policy for {host.label}.
          </DialogDescription>
        </DialogHeader>
        <div className="px-5 py-4 text-sm leading-6 text-muted-foreground sm:px-6">
          Cancel keeps your current role edits. Reset restores the selected preset.
        </div>
        <DialogFooter className="border-t bg-muted/30 px-5 py-4 sm:px-6">
          <DialogClose render={<Button type="button" variant="outline" />}>
            Cancel
          </DialogClose>
          <Button type="button" variant="destructive" aria-label="Confirm reset roles" onClick={onConfirm}>
            <ArrowCounterClockwiseIcon data-icon="inline-start" aria-hidden="true" />
            Reset roles
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function PresetField({ preset, onPreset }: {
  preset: PresetSelectionState;
  onPreset: (preset: PresetId) => void;
}) {
  return (
    <Field>
      <div className="flex items-center justify-between gap-3">
        <FieldTitle>Team preset</FieldTitle>
        {preset.selected === "custom" && <Badge variant="secondary">Custom</Badge>}
      </div>
      <ToggleGroup
        aria-label="Team preset"
        value={preset.selected === "custom" ? [] : [preset.selected]}
        onValueChange={(values) => values[0] && onPreset(values[0] as PresetId)}
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
        {preset.selected === "custom" ? "Per-role choices override the preset." : PRESETS[preset.selected].short}
      </FieldDescription>
    </Field>
  );
}

function GeneratorSummaryPanel({ childRoleCount, commands, config, copiedCommandId, hasHostManagedParent, host, onboarding, outputRoleIds, setup, onCopyCommand }: {
  childRoleCount: number;
  commands: ReturnType<typeof lifecycleCommands>;
  config: GeneratorConfigState;
  copiedCommandId: string | null;
  hasHostManagedParent: boolean;
  host: (typeof HOSTS)[HostId];
  onboarding: OnboardingState;
  outputRoleIds: RoleId[];
  setup: ReturnType<typeof setupSpec>;
  onCopyCommand: (id: string, command: string) => void;
}) {
  return (
    <Card className="min-w-0 lg:sticky lg:top-6">
      <CardHeader>
        <CardTitle>Your {host.label} team</CardTitle>
        <CardDescription>
          {hasHostManagedParent
            ? `${childRoleCount} generated child ${childRoleCount === 1 ? "role" : "roles"} · host-managed parent`
            : `${outputRoleIds.length} focused ${outputRoleIds.length === 1 ? "role" : "roles"}`}
          {" · "}
          {config.integration === "planr" ? "Planr setup spec" : "standalone setup spec"}
        </CardDescription>
      </CardHeader>
      <CardContent>
        <Tabs defaultValue="team">
          <TabsList className="w-full">
            <TabsTrigger value="team">Team</TabsTrigger>
            <TabsTrigger value="commands">Commands</TabsTrigger>
            <TabsTrigger value="spec">Spec</TabsTrigger>
          </TabsList>
          <TabsContent value="team" className="pt-3">
            <TeamSummary config={config} hasHostManagedParent={hasHostManagedParent} />
          </TabsContent>
          <TabsContent value="commands" className="pt-3">
            <CommandSummary commands={commands} copiedCommandId={copiedCommandId} onCopyCommand={onCopyCommand} />
          </TabsContent>
          <TabsContent value="spec" className="pt-3">
            <pre className="max-h-80 overflow-auto rounded-sm bg-muted p-3 text-[0.68rem] leading-5 text-muted-foreground">
              {JSON.stringify(setup, null, 2)}
            </pre>
          </TabsContent>
        </Tabs>
      </CardContent>
      <CardFooter className="flex-col items-stretch gap-2">
        <AgentTeamDialog onboarding={onboarding} />
        <p className="pt-1 text-center text-[0.7rem] leading-5 text-muted-foreground">
          Preview before apply, run doctor to check the host, and review every repository-local change before confirming setup.
        </p>
      </CardFooter>
    </Card>
  );
}

function TeamSummary({ config, hasHostManagedParent }: {
  config: GeneratorConfigState;
  hasHostManagedParent: boolean;
}) {
  return (
    <div className="flex flex-col gap-3">
      {hasHostManagedParent && (
        <div className="grid grid-cols-[1.5rem_1fr] gap-3">
          <span className="flex size-6 items-center justify-center bg-muted text-[0.7rem]">P</span>
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <p className="font-medium">{ROLES.orchestrator.label}</p>
              <Badge variant="secondary">Host managed</Badge>
            </div>
            <p className="truncate text-muted-foreground">
              {config.assignments.orchestrator.effort ? `${config.assignments.orchestrator.effort} · ` : ""}not written to Spec
            </p>
          </div>
        </div>
      )}
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
      {!hasHostManagedParent && (
        <div className="grid grid-cols-[1.5rem_1fr] gap-3">
          <span className="flex size-6 items-center justify-center bg-muted text-[0.7rem]">{config.roles.length + 1}</span>
          <div className="min-w-0">
            <p className="font-medium">{ROLES.orchestrator.label}</p>
            <p className="truncate text-muted-foreground">
              {config.assignments.orchestrator.model}{config.assignments.orchestrator.effort ? ` · ${config.assignments.orchestrator.effort}` : ""}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}

function CommandSummary({ commands, copiedCommandId, onCopyCommand }: {
  commands: ReturnType<typeof lifecycleCommands>;
  copiedCommandId: string | null;
  onCopyCommand: (id: string, command: string) => void;
}) {
  return (
    <ol className="flex min-w-0 flex-col gap-3">
      {commands.map((entry) => (
        <li key={entry.id} className="min-w-0 rounded-sm bg-muted px-2 py-2">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <p className="text-xs font-medium text-foreground">{entry.title}</p>
              <p className="mt-0.5 text-[0.68rem] leading-4 text-muted-foreground">{entry.description}</p>
            </div>
            <Button
              type="button"
              size="icon-xs"
              variant="ghost"
              className="shrink-0"
              aria-label={`Copy ${entry.title} command`}
              onClick={() => onCopyCommand(entry.id, entry.command)}
            >
              {copiedCommandId === entry.id
                ? <CheckIcon aria-hidden="true" />
                : <CopyIcon aria-hidden="true" />}
            </Button>
          </div>
          <code className="mt-1.5 block max-w-full truncate font-mono text-[0.68rem] leading-5 text-muted-foreground">
            {truncateCommandPreview(entry.command)}
          </code>
        </li>
      ))}
    </ol>
  );
}

function ConnectedChildCard({ children, isLast }: { children: React.ReactNode; isLast: boolean }) {
  return (
    <div
      className={cn(
        "relative pl-6 before:absolute before:left-2 before:top-0 before:w-px before:bg-border",
        isLast ? "before:h-6" : "before:bottom-[-0.75rem]",
      )}
    >
      <span aria-hidden="true" className="absolute left-2 top-6 h-px w-4 bg-border" />
      {children}
    </div>
  );
}

function ParentRecommendationCard({ config, preset }: {
  config: ReturnType<typeof createConfig>;
  preset: PresetId;
}) {
  const recommendation = primaryRecommendation(config);
  const effortCopy = parentRecommendationEffortCopy(config, preset);

  return (
    <Card size="sm" className="border-dashed bg-muted/30">
      <CardHeader>
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <CardTitle>{ROLES[recommendation.id].label}</CardTitle>
            <CardDescription>{ROLES[recommendation.id].short}</CardDescription>
          </div>
          <Badge variant="secondary" className="shrink-0">Host managed</Badge>
        </div>
      </CardHeader>
      <CardContent>
        <p className="text-sm leading-6 text-muted-foreground">{effortCopy}</p>
      </CardContent>
    </Card>
  );
}

function RoleCard({ role, config, hostCatalog, canRemove, onRemove, onModel, onEffort }: {
  role: ChildRoleId;
  config: ReturnType<typeof createConfig>;
  hostCatalog: HostCatalog;
  canRemove: boolean;
  onRemove: () => void;
  onModel: (model: string) => void;
  onEffort: (effort: string) => void;
}) {
  const host = HOSTS[config.host];
  const models = hostCatalog[config.host].models;
  const assignment = config.assignments[role];
  const model = models.find((candidate) => candidate.id === assignment.model)!;
  const family = modelFamilyInfo(model.id);
  const removeLabel = canRemove
    ? `Remove ${ROLES[role].label}`
    : `Cannot remove ${ROLES[role].label}; at least one child role is required`;
  return (
    <Card size="sm">
      <CardHeader>
        <div className="min-w-0">
          <CardTitle>{ROLES[role].label}</CardTitle>
          <CardDescription>{ROLES[role].short}</CardDescription>
          <p className="mt-1 truncate text-[0.72rem] leading-5 text-muted-foreground">
            {model.label}{assignment.effort ? ` · ${assignment.effort}` : ""}
          </p>
        </div>
        <CardAction className="flex items-center gap-1">
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            aria-label={removeLabel}
            disabled={!canRemove}
            onClick={onRemove}
          >
            <TrashIcon aria-hidden="true" />
          </Button>
        </CardAction>
      </CardHeader>
      {!canRemove && (
        <CardContent className="pt-0">
          <p className="text-xs leading-5 text-muted-foreground">At least one child role is required.</p>
        </CardContent>
      )}
      <CardContent>
        <FieldGroup>
          <div className="flex flex-wrap items-start gap-4">
              <Field className="min-w-0 flex-1">
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
                      placeholder="Search latest models..."
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
                      const optionFamily = modelFamilyInfo(option.id);
                      if (option.disabledReason) {
                        return (
                          <Tooltip key={option.id}>
                            <TooltipTrigger render={<span className="inline-block w-fit" />}>
                              <ToggleGroupItem value={option.id} disabled className="gap-1.5">
                                {optionFamily && <ModelStrengthKugeln family={optionFamily.id} />}
                                {option.label}
                              </ToggleGroupItem>
                            </TooltipTrigger>
                            <TooltipContent>
                              <p>{option.disabledReason}</p>
                            </TooltipContent>
                          </Tooltip>
                        );
                      }
                      return (
                        <ToggleGroupItem key={option.id} value={option.id} className="gap-1.5">
                          {optionFamily && <ModelStrengthKugeln family={optionFamily.id} />}
                          {option.label}
                        </ToggleGroupItem>
                      );
                    })}
                  </ToggleGroup>
                )}
                {family && (
                  <FieldDescription>{family.short}</FieldDescription>
                )}
              </Field>
              {host.effortLabel && model.efforts.length > 0 && assignment.effort && (
                <Field className="w-auto shrink-0">
                  <FieldTitle>{host.effortLabel}</FieldTitle>
                  <EffortStrengthPicker
                    label={host.effortLabel}
                    efforts={model.efforts}
                    value={assignment.effort}
                    onValueChange={onEffort}
                    valueNote={
                      config.host === "codex" && assignment.effort === "max"
                        ? "Max gives one Codex agent the largest reasoning budget. It may need to be enabled in Codex app settings; Ultra additionally enables automatic delegation."
                        : undefined
                    }
                    aria-label={`${ROLES[role].label} ${host.effortLabel}`}
                  />
                </Field>
              )}
          </div>
        </FieldGroup>
      </CardContent>
    </Card>
  );
}
