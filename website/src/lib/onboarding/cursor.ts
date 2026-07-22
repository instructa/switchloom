import type { ProviderOnboardingTemplate } from "./types";

export const cursorOnboarding = {
  host: "cursor",
  icon: "/brand/cursor.svg",
  title: "Set up your Cursor agent team",
  description: "Switchloom installs native Cursor child agents while Cursor remains the parent runtime and model-selection authority.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Check Cursor Agent",
      description: "Use the current Cursor Agent CLI and confirm it launches in the target repository.",
      command: { kind: "literal", value: "cursor-agent --version" },
    },
    {
      id: "project",
      title: "Use repository-native agents",
      description: "cd into your project first. Switchloom sets up the Cursor child-agent files there and does not touch your global Cursor settings.",
    },
    {
      id: "install",
      title: "Apply the team from your project",
      description: "Copy the command and run it in a terminal from that project directory.",
      command: { kind: "apply" },
    },
    {
      id: "activate",
      title: "Start a fresh session and verify",
      description: "Restart Cursor Agent so it discovers the new project agents, then run doctor. Effective model claims remain advisory until Cursor exposes stronger receipts.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
