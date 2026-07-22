import { useState } from "react";
import { ArrowRightIcon, CheckIcon, CopyIcon, TerminalWindowIcon, UsersThreeIcon } from "@phosphor-icons/react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import type { ResolvedProviderOnboarding } from "@/lib/onboarding";

export function AgentTeamDialog({ onboarding }: { onboarding: ResolvedProviderOnboarding }) {
  const [copiedStep, setCopiedStep] = useState<string | null>(null);
  const [copyFailedStep, setCopyFailedStep] = useState<string | null>(null);
  const [showInstallCommand, setShowInstallCommand] = useState(false);

  async function copyCommand(stepId: string, command: string) {
    try {
      await navigator.clipboard.writeText(command);
      setCopyFailedStep(null);
      setCopiedStep(stepId);
      window.setTimeout(() => setCopiedStep((current) => current === stepId ? null : current), 1400);
    } catch {
      setCopiedStep(null);
      setCopyFailedStep(stepId);
    }
  }

  return (
    <Dialog>
      <DialogTrigger render={<Button size="lg" className="w-full" />}>
        <UsersThreeIcon data-icon="inline-start" aria-hidden="true" />
        Get your agent team
        <ArrowRightIcon data-icon="inline-end" aria-hidden="true" />
      </DialogTrigger>
      <DialogContent className="max-h-[min(90vh,52rem)] overflow-y-auto p-0 sm:max-w-2xl">
        <DialogHeader className="border-b px-5 py-5 pr-12 sm:px-6">
          <div className="mb-2 flex items-center gap-2">
            <span className="flex size-8 items-center justify-center border bg-background">
              <img src={onboarding.icon} alt="" aria-hidden="true" className="size-4 object-contain" />
            </span>
            <Badge variant={onboarding.status === "certified" ? "default" : "secondary"}>
              {onboarding.status}
            </Badge>
          </div>
          <DialogTitle className="text-lg sm:text-xl">{onboarding.title}</DialogTitle>
          <DialogDescription className="max-w-xl text-sm leading-6">
            {onboarding.description}
          </DialogDescription>
        </DialogHeader>

        <ol className="flex flex-col px-5 pb-5 sm:px-6 sm:pb-6">
          {onboarding.steps.map((step, index) => (
            <li
              key={step.id}
              className={cn(
                "grid grid-cols-[2rem_minmax(0,1fr)] gap-3 border-b py-5 last:border-b-0 last:pb-0",
                step.id === "install" && "-mx-3 border border-primary/20 bg-primary/[0.035] px-3",
              )}
            >
              <span className={cn(
                "flex size-8 items-center justify-center border bg-muted text-xs font-medium",
                step.id === "install" && "border-primary bg-primary text-primary-foreground",
              )}>
                {index + 1}
              </span>
              <div className="min-w-0">
                <h3 className="font-heading text-sm font-medium">{step.title}</h3>
                <p className="mt-1 text-xs leading-5 text-muted-foreground sm:text-sm sm:leading-6">
                  {step.description}
                </p>
                {step.command && (
                  step.id === "install" ? (
                    <div className="mt-3 flex flex-col items-start gap-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Button
                          type="button"
                          size="sm"
                          aria-label={`Copy ${step.title} command`}
                          onClick={() => copyCommand(step.id, step.command!)}
                        >
                          {copiedStep === step.id
                            ? <CheckIcon aria-hidden="true" />
                            : <CopyIcon aria-hidden="true" />}
                          <span>{copiedStep === step.id ? "Copied" : "Copy command"}</span>
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          aria-expanded={showInstallCommand}
                          aria-controls="install-command-preview"
                          onClick={() => setShowInstallCommand((open) => !open)}
                        >
                          {showInstallCommand ? "Hide command" : "View command"}
                        </Button>
                      </div>
                      {showInstallCommand && (
                        <div id="install-command-preview" className="max-w-full min-w-0 overflow-hidden border bg-muted/60 p-2">
                          <code className="block max-w-full select-all break-all whitespace-pre-wrap font-mono text-[0.68rem] leading-5 sm:text-xs">
                            {step.command}
                          </code>
                        </div>
                      )}
                      {copyFailedStep === step.id && (
                        <p role="status" className="text-[0.68rem] text-muted-foreground">
                          Clipboard access failed. Try again or copy the command from the Commands tab.
                        </p>
                      )}
                    </div>
                  ) : (
                    <div className="mt-3 border bg-muted/60">
                      <div className="flex items-center gap-2 p-2">
                        <TerminalWindowIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
                        <code className="min-w-0 flex-1 select-all overflow-x-auto whitespace-pre font-mono text-[0.68rem] leading-5 sm:text-xs">
                          {step.command}
                        </code>
                        <Button
                          type="button"
                          size="icon-sm"
                          variant="ghost"
                          aria-label={`Copy ${step.title} command`}
                          onClick={() => copyCommand(step.id, step.command!)}
                        >
                          {copiedStep === step.id
                            ? <CheckIcon aria-hidden="true" />
                            : <CopyIcon aria-hidden="true" />}
                        </Button>
                      </div>
                      {copyFailedStep === step.id && (
                        <p role="status" className="border-t px-2 py-1.5 text-[0.68rem] text-muted-foreground">
                          Clipboard access failed. Select the command above and copy it manually.
                        </p>
                      )}
                    </div>
                  )
                )}
              </div>
            </li>
          ))}
        </ol>
      </DialogContent>
    </Dialog>
  );
}
