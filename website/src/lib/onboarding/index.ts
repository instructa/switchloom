import { type HostId } from "../generator";

import { claudeCodeOnboarding } from "./claude-code";
import { codexOnboarding } from "./codex";
import { cursorOnboarding } from "./cursor";
import { opencodeOnboarding } from "./opencode";
import { piOnboarding } from "./pi";
import type {
  OnboardingCommand,
  ProviderOnboardingTemplate,
  ResolvedOnboardingStep,
  ResolvedProviderOnboarding,
} from "./types";

export const PROVIDER_ONBOARDING = {
  codex: codexOnboarding,
  cursor: cursorOnboarding,
  "claude-code": claudeCodeOnboarding,
  opencode: opencodeOnboarding,
  pi: piOnboarding,
} satisfies Record<HostId, ProviderOnboardingTemplate>;

function resolveCommand(command: OnboardingCommand, host: HostId, applyCommand: string) {
  switch (command.kind) {
    case "apply":
      return applyCommand;
    case "doctor":
      return `npx switchloom doctor ${host}`;
    case "literal":
      return command.value;
  }
}

export function providerOnboarding(host: HostId, applyCommand: string): ResolvedProviderOnboarding {
  const template = PROVIDER_ONBOARDING[host];
  return {
    ...template,
    steps: template.steps.map((step): ResolvedOnboardingStep => {
      const { command, ...content } = step;
      return {
        ...content,
        ...(command ? { command: resolveCommand(command, host, applyCommand) } : {}),
      };
    }),
  };
}

export type { ProviderOnboardingTemplate, ResolvedProviderOnboarding } from "./types";
