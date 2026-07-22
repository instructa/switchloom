import { useId } from "react";

import { cn } from "@/lib/utils";
import type { ModelFamilyId } from "@/lib/model-family";

const FAMILY_LOOK: Record<
  ModelFamilyId,
  {
    strength: 1 | 2 | 3;
    highlight: string;
    mid: string;
    core: string;
    rim: string;
  }
> = {
  luna: {
    strength: 1,
    highlight: "#f4f4f5",
    mid: "#c8cdd4",
    core: "#6b7280",
    rim: "#9ca3af",
  },
  terra: {
    strength: 2,
    highlight: "#7ef0ff",
    mid: "#2f7dff",
    core: "#0b3d8c",
    rim: "#3aa0ff",
  },
  sol: {
    strength: 3,
    highlight: "#ffc078",
    mid: "#e07020",
    core: "#6a2a0c",
    rim: "#f08a3a",
  },
};

/** One celestial mass mark with GPT-5.6 family color + digit texture. */
export function ModelStrengthKugeln({
  family,
  className,
  title,
}: {
  family: ModelFamilyId;
  className?: string;
  title?: string;
}) {
  const uid = useId().replaceAll(":", "");
  const look = FAMILY_LOOK[family];
  const size = 12 + look.strength * 2;
  const radius = 2.6 + look.strength * 1.05;
  const gradientId = `${uid}-grad`;
  const textureId = `${uid}-tex`;

  return (
    <svg
      viewBox="0 0 16 16"
      width={size}
      height={size}
      aria-hidden={title ? undefined : true}
      role={title ? "img" : undefined}
      className={cn("shrink-0", className)}
    >
      {title ? <title>{title}</title> : null}
      <defs>
        <radialGradient id={gradientId} cx="32%" cy="28%" r="72%">
          <stop offset="0%" stopColor={look.highlight} />
          <stop offset="48%" stopColor={look.mid} />
          <stop offset="100%" stopColor={look.core} />
        </radialGradient>
        <pattern id={textureId} width="4" height="4" patternUnits="userSpaceOnUse">
          <text x="0.2" y="2.8" fontSize="2.4" fill={look.highlight} fillOpacity="0.55" fontFamily="ui-monospace, monospace">
            5
          </text>
          <text x="2" y="3.6" fontSize="2.1" fill={look.highlight} fillOpacity="0.35" fontFamily="ui-monospace, monospace">
            6
          </text>
        </pattern>
      </defs>
      <circle cx="8" cy="8" r={radius} fill={`url(#${gradientId})`} />
      <circle cx="8" cy="8" r={radius} fill={`url(#${textureId})`} style={{ mixBlendMode: "soft-light" }} />
      <circle
        cx="8"
        cy="8"
        r={radius}
        fill="none"
        stroke={look.rim}
        strokeOpacity={0.85}
        strokeWidth={1}
      />
      <circle
        cx={8 - radius * 0.35}
        cy={8 - radius * 0.35}
        r={radius * 0.28}
        fill={look.highlight}
        fillOpacity={0.35}
      />
    </svg>
  );
}
