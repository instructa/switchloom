import type { ProviderOnboardingTemplate } from "./types";

export const codexOnboarding = {
  host: "codex",
  icon: "/brand/codex.svg",
  title: "Set up your Codex agent team",
  description: "Switchloom installs native V2 child roles into this repository. Codex remains the parent orchestrator and permission authority.",
  status: "certified",
  steps: [
    {
      id: "requirements",
      title: "Check your Codex version",
      description: "Use Codex 0.145.0 or newer.",
      command: { kind: "literal", value: "codex --version" },
    },
    {
      id: "project",
      title: "Keep configuration project-local",
      description: "cd into your project first. Switchloom sets up the Codex child-role files there and does not touch your global Codex settings.",
    },
    {
      id: "install",
      title: "Apply the team from your project",
      description: "Copy the command and run it in a terminal from that project directory.",
      command: { kind: "apply" },
    },
    {
      id: "activate",
      title: "Trust, restart, and verify",
      description: "Trust the repository if Codex asks, start a fresh Codex session, then run doctor. Ask Codex to delegate when you want a specific handoff.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
