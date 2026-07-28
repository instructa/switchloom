import type { ProviderOnboardingTemplate } from "./types";

export const opencodeOnboarding = {
  host: "opencode",
  icon: "/brand/opencode.svg",
  title: "Set up your OpenCode agent team",
  description: "Switchloom installs provider-qualified OpenCode child agents with explicit task permissions. Provider credentials remain in your OpenCode setup, never the recipe.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Connect a provider in OpenCode",
      description: "Use OpenCode's runtime-owned provider connection flow before selecting provider-qualified child models.",
      command: { kind: "literal", value: "/connect" },
    },
    {
      id: "project",
      title: "Keep agents in the repository",
      description: "cd into your project first. Switchloom sets up child-agent files there and does not touch provider credentials.",
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
