import type { ProviderOnboardingTemplate } from "./types";

export const claudeCodeOnboarding = {
  host: "claude-code",
  icon: "/brand/claude.svg",
  title: "Set up your Claude Code agent team",
  description: "Switchloom installs Claude Code project child subagents. Claude Code remains the parent runtime.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Check Claude Code",
      description: "Use a current Claude Code CLI with project subagent support.",
      command: { kind: "literal", value: "claude --version" },
    },
    {
      id: "project",
      title: "Keep agents in the repository",
      description: "cd into your project first. Switchloom sets up the Claude child-agent files there and does not touch your global Claude settings.",
    },
    {
      id: "install",
      title: "Apply the team from your project",
      description: "Copy the command and run it in a terminal from that project directory.",
      command: { kind: "apply" },
    },
    {
      id: "activate",
      title: "Restart Claude Code and verify",
      description: "Start a fresh Claude Code session so project subagents are rediscovered, then run doctor before relying on the setup.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
