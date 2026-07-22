import type { ProviderOnboardingTemplate } from "./types";

export const opencodeOnboarding = {
  host: "opencode",
  icon: "/brand/opencode.svg",
  title: "Set up your OpenCode agent team",
  description: "Switchloom installs provider-qualified OpenCode child agents with explicit task permissions.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Check OpenCode",
      description: "Use a current OpenCode CLI with project agent and Task support.",
      command: { kind: "literal", value: "opencode --version" },
    },
    {
      id: "project",
      title: "Keep agents in the repository",
      description: "cd into your project first. Switchloom sets up the OpenCode child-agent files there and does not touch your global OpenCode settings.",
    },
    {
      id: "install",
      title: "Apply the team from your project",
      description: "Copy the command and run it in a terminal from that project directory.",
      command: { kind: "apply" },
    },
    {
      id: "activate",
      title: "Restart OpenCode and verify",
      description: "Start a fresh OpenCode session so project agents are rediscovered, then run doctor before relying on the setup.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
