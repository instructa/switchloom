import type { ProviderOnboardingTemplate } from "./types";

export const piOnboarding = {
  host: "pi",
  icon: "/brand/pi.svg",
  title: "Set up your Pi agent team",
  description: "Your active main Pi session is the Orchestrator. Pi owns provider login; Switchloom installs Pi Subagents, child role agents, and a sequential workflow.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Sign in from the active Pi session",
      description: "Use Pi's runtime-owned login flow before selecting provider-qualified child models.",
      command: { kind: "literal", value: "/login" },
    },
    {
      id: "project",
      title: "Install Pi Subagents",
      description: "Run this once, then keep the generated role agents and chain in your repository.",
      command: { kind: "literal", value: "pi install npm:pi-subagents" },
    },
    {
      id: "install",
      title: "Wait for live certification",
      description: "This generated workflow is Experimental. Do not apply it as certified support until a credentialed requested/effective-model receipt is retained.",
    },
    {
      id: "activate",
      title: "Inspect the extension workflow",
      description: "Review .pi/settings.json, .pi/agents, and the generated chain before running it from the active Pi session.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
