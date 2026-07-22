import { useId, useRef } from "react";
import { CaretDownIcon, InfoIcon } from "@phosphor-icons/react";

import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { effortHint, formatEffortLabel } from "@/lib/model-family";

export function EffortStrengthPicker({
  label,
  efforts,
  value,
  onValueChange,
  valueNote,
  "aria-label": ariaLabel,
  className,
}: {
  label: string;
  efforts: readonly string[];
  value: string;
  onValueChange: (effort: string) => void;
  valueNote?: string;
  "aria-label": string;
  className?: string;
}) {
  const labelId = useId();
  const trackRef = useRef<HTMLDivElement>(null);
  const index = Math.max(0, efforts.indexOf(value));
  const max = Math.max(efforts.length - 1, 1);
  const percent = efforts.length <= 1 ? 0 : (index / max) * 100;
  const hint = effortHint(value);
  const longestLabel = efforts.reduce((longest, effort) => {
    const next = formatEffortLabel(effort);
    return next.length > longest.length ? next : longest;
  }, formatEffortLabel(value));

  function setIndex(next: number) {
    const clamped = Math.min(Math.max(next, 0), efforts.length - 1);
    const effort = efforts[clamped];
    if (effort) onValueChange(effort);
  }

  function onTrackPointer(clientX: number) {
    const track = trackRef.current;
    if (!track || efforts.length <= 1) return;
    const rect = track.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.min(Math.max((clientX - rect.left) / rect.width, 0), 1);
    setIndex(Math.round(ratio * max));
  }

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <Popover>
        <PopoverTrigger
          render={
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="justify-between gap-2"
              aria-label={ariaLabel}
            />
          }
        >
          <span className="relative inline-grid justify-items-start">
            <span className="invisible col-start-1 row-start-1" aria-hidden="true">
              {longestLabel}
            </span>
            <span className="col-start-1 row-start-1 flex items-center gap-1">
              {formatEffortLabel(value)}
              {valueNote ? (
                <Tooltip>
                  <TooltipTrigger render={<span className="inline-flex cursor-help text-muted-foreground" />}>
                    <InfoIcon aria-label={`${formatEffortLabel(value)} information`} />
                  </TooltipTrigger>
                  <TooltipContent className="max-w-xs text-pretty">{valueNote}</TooltipContent>
                </Tooltip>
              ) : null}
            </span>
          </span>
          <CaretDownIcon data-icon="inline-end" aria-hidden="true" />
        </PopoverTrigger>
        <PopoverContent
          align="end"
          side="bottom"
          className="w-[min(18rem,calc(100vw-2rem))] gap-3 p-3 sm:w-72"
        >
          <PopoverHeader>
            <PopoverTitle id={labelId}>{label}</PopoverTitle>
            <PopoverDescription className="h-10 overflow-hidden">
              <span className="font-medium text-foreground">{formatEffortLabel(value)}</span>
              {hint ? ` — ${hint}` : null}
            </PopoverDescription>
          </PopoverHeader>

          <div
            role="slider"
            tabIndex={0}
            aria-labelledby={labelId}
            aria-valuemin={0}
            aria-valuemax={max}
            aria-valuenow={index}
            aria-valuetext={formatEffortLabel(value)}
            className="relative touch-none select-none py-3 outline-none focus-visible:ring-1 focus-visible:ring-ring"
            onKeyDown={(event) => {
              if (event.key === "ArrowRight" || event.key === "ArrowUp") {
                event.preventDefault();
                setIndex(index + 1);
              } else if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
                event.preventDefault();
                setIndex(index - 1);
              } else if (event.key === "Home") {
                event.preventDefault();
                setIndex(0);
              } else if (event.key === "End") {
                event.preventDefault();
                setIndex(efforts.length - 1);
              }
            }}
            onPointerDown={(event) => {
              event.currentTarget.setPointerCapture(event.pointerId);
              onTrackPointer(event.clientX);
            }}
            onPointerMove={(event) => {
              if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
              onTrackPointer(event.clientX);
            }}
          >
            <div ref={trackRef} className="relative mx-1 h-2 rounded-full bg-muted">
              <div
                className="absolute inset-y-0 left-0 rounded-full bg-primary"
                style={{ width: `${percent}%` }}
              />
              {efforts.map((effort, effortIndex) => {
                const left = efforts.length <= 1 ? 0 : (effortIndex / max) * 100;
                return (
                  <span
                    key={effort}
                    aria-hidden="true"
                    className={cn(
                      "absolute top-1/2 size-1.5 -translate-x-1/2 -translate-y-1/2 rounded-full",
                      effortIndex <= index ? "bg-primary-foreground/70" : "bg-foreground/25",
                    )}
                    style={{ left: `${left}%` }}
                  />
                );
              })}
              <span
                aria-hidden="true"
                className="absolute top-1/2 size-4 -translate-x-1/2 -translate-y-1/2 rounded-full bg-background shadow-sm ring-2 ring-primary"
                style={{ left: `${percent}%` }}
              />
            </div>
          </div>

          <div className="flex justify-between gap-2 text-[0.68rem] text-muted-foreground">
            <span>{formatEffortLabel(efforts[0] ?? value)}</span>
            <span>{formatEffortLabel(efforts[efforts.length - 1] ?? value)}</span>
          </div>
        </PopoverContent>
      </Popover>
    </div>
  );
}
