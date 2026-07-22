import type { ProviderOnboardingTemplate } from "./types";

export const piOnboarding = {
  host: "pi",
  icon: "/brand/pi.svg",
  title: "Set up your Pi agent team",
  description: "Switchloom installs isolated external-runner workflows. Pi remains a process boundary rather than a native child-thread runtime.",
  status: "experimental",
  steps: [
    {
      id: "requirements",
      title: "Check Pi",
      description: "Use a current Pi CLI with print-mode provider, model, and thinking controls.",
      command: { kind: "literal", value: "pi --version" },
    },
    {
      id: "project",
      title: "Keep workflows in the repository",
      description: "cd into your project first. Switchloom sets up the Pi workflow files there and does not touch your global Pi settings.",
    },
    {
      id: "install",
      title: "Apply the team from your project",
      description: "Copy the command and run it in a terminal from that project directory.",
      command: { kind: "apply" },
    },
    {
      id: "activate",
      title: "Inspect and verify the runner",
      description: "Review the generated workflow before execution, then run doctor. Pi roles execute as isolated processes, not native subagents.",
      command: { kind: "doctor" },
    },
  ],
} satisfies ProviderOnboardingTemplate;
