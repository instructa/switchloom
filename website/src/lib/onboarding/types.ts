import type { HostId } from "../generator";

export type OnboardingCommand =
  | { kind: "apply" }
  | { kind: "doctor" }
  | { kind: "literal"; value: string };

export type OnboardingStep = {
  id: "requirements" | "project" | "install" | "activate";
  title: string;
  description: string;
  command?: OnboardingCommand;
};

export type ProviderOnboardingTemplate = {
  host: HostId;
  icon: string;
  title: string;
  description: string;
  status: "certified" | "experimental";
  steps: readonly OnboardingStep[];
};

export type ResolvedOnboardingStep = Omit<OnboardingStep, "command"> & {
  command?: string;
};

export type ResolvedProviderOnboarding = Omit<ProviderOnboardingTemplate, "steps"> & {
  steps: readonly ResolvedOnboardingStep[];
};
