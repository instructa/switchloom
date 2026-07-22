import { isValidElement, type ReactElement, type ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import catalog from "../../data/catalog.json";
import Generator from "./Generator";
import { hostCatalogFrom, setupTransportFrom } from "../lib/generator";

const reactState = vi.hoisted(() => ({
  cursor: 0,
  values: [] as unknown[],
  reset() {
    this.cursor = 0;
    this.values = [];
  },
  rewind() {
    this.cursor = 0;
  },
  useState<T>(initial: T | (() => T)) {
    const index = this.cursor;
    this.cursor += 1;
    if (!(index in this.values)) {
      this.values[index] = typeof initial === "function" ? (initial as () => T)() : initial;
    }
    const setValue = (next: T | ((current: T) => T)) => {
      const current = this.values[index] as T;
      this.values[index] = typeof next === "function" ? (next as (current: T) => T)(current) : next;
    };
    return [this.values[index] as T, setValue] as const;
  },
}));

vi.mock("react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("react")>();
  return {
    ...actual,
    useMemo: <T>(factory: () => T) => factory(),
    useState: <T>(initial: T | (() => T)) => reactState.useState(initial),
  };
});

const hostCatalog = hostCatalogFrom(catalog);
const setupTransport = setupTransportFrom(catalog);

type TestRenderableComponent = (props: Record<string, unknown>) => ReactNode;

function renderGenerator() {
  reactState.rewind();
  return Generator({ hostCatalog, setupTransport });
}

function elementProps(node: ReactNode) {
  return isValidElement(node) ? (node as ReactElement<Record<string, unknown>>).props : undefined;
}

function isTestRenderableComponent(type: unknown): type is TestRenderableComponent {
  return typeof type === "function" && [
    "CommandSummary",
    "ConnectedChildCard",
    "GeneratorConfigPanel",
    "GeneratorSummaryPanel",
    "IntegrationFieldSet",
    "ParentRecommendationCard",
    "PresetField",
    "ResetRolesDialog",
    "RoleCard",
    "RoleTuningFieldSet",
    "RuntimeFieldSet",
    "TeamSummary",
  ].includes(type.name);
}

function visibleText(node: ReactNode): string {
  if (node === null || node === undefined || typeof node === "boolean") {
    return "";
  }
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }
  if (Array.isArray(node)) {
    return node.map(visibleText).join("");
  }
  if (!isValidElement(node)) {
    return "";
  }

  const element = node as ReactElement<Record<string, unknown>>;
  const type = element.type;
  if (isTestRenderableComponent(type)) {
    return visibleText(type(element.props));
  }

  return visibleText(element.props.children as ReactNode);
}

function findElement(node: ReactNode, predicate: (element: ReactElement<Record<string, unknown>>) => boolean): ReactElement<Record<string, unknown>> | null {
  if (node === null || node === undefined || typeof node === "boolean" || typeof node === "string" || typeof node === "number") {
    return null;
  }
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findElement(child, predicate);
      if (found) {
        return found;
      }
    }
    return null;
  }
  if (!isValidElement(node)) {
    return null;
  }

  const element = node as ReactElement<Record<string, unknown>>;
  if (predicate(element)) {
    return element;
  }

  const foundInRenderedControl = findElement(element.props.render as ReactNode, predicate);
  if (foundInRenderedControl) {
    return foundInRenderedControl;
  }
  if (isTestRenderableComponent(element.type)) {
    const foundInRenderedComponent = findElement(element.type(element.props), predicate);
    if (foundInRenderedComponent) {
      return foundInRenderedComponent;
    }
  }
  return findElement(element.props.children as ReactNode, predicate);
}

function findByAriaLabel(node: ReactNode, label: string): ReactElement<Record<string, unknown>> {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findByAriaLabelOrNull(child, label);
      if (found) {
        return found;
      }
    }
  }

  const found = findByAriaLabelOrNull(node, label);
  if (!found) {
    throw new Error(`Could not find element with aria-label ${label}`);
  }
  return found;
}

function findByAriaLabelOrNull(node: ReactNode, label: string): ReactElement<Record<string, unknown>> | null {
  if (node === null || node === undefined || typeof node === "boolean" || typeof node === "string" || typeof node === "number") {
    return null;
  }
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findByAriaLabelOrNull(child, label);
      if (found) {
        return found;
      }
    }
    return null;
  }
  if (!isValidElement(node)) {
    return null;
  }

  const props = elementProps(node)!;
  if (props["aria-label"] === label) {
    return node as ReactElement<Record<string, unknown>>;
  }
  const renderedControl = props.render as ReactNode;
  const foundInRenderedControl = findByAriaLabelOrNull(renderedControl, label);
  if (foundInRenderedControl) {
    return foundInRenderedControl;
  }
  if (isTestRenderableComponent(node.type)) {
    return findByAriaLabelOrNull(node.type(props), label);
  }
  return findByAriaLabelOrNull(props.children as ReactNode, label);
}

function selectToggle(label: string, value: string) {
  const control = findByAriaLabel(renderGenerator(), label);
  const onValueChange = control.props.onValueChange;
  if (typeof onValueChange !== "function") {
    throw new Error(`${label} is not selectable`);
  }
  onValueChange([value]);
}

describe("Generator parent recommendation card", () => {
  beforeEach(() => {
    reactState.reset();
  });

  it("renders the non-editable parent card as host managed with Balanced effort copy", () => {
    const text = visibleText(renderGenerator());

    expect(text).toContain("Orchestrator");
    expect(text).toContain("Host managed");
    expect(text).toContain("Set Codex reasoning to Medium.");
    expect(text).not.toContain("Not included");
    expect(text).not.toContain("Balanced recommends Sol");
  });

  it("counts generated children separately from the host-managed parent in the summary", () => {
    const text = visibleText(renderGenerator());

    expect(text).toContain("3 generated child roles");
    expect(text).toContain("host-managed parent");
    expect(text).toContain("not written to Spec");
    expect(text).not.toContain("4 focused roles");
  });

  it("shows host-managed parent effort without rendering the parent model in the Team summary", () => {
    selectToggle("AI agent", "cursor");
    const text = visibleText(renderGenerator());

    expect(text).toContain("Orchestrator");
    expect(text).toContain("Host managed");
    expect(text).toContain("high · not written to Spec");
    expect(text).not.toContain("fable-5");
  });

  it("updates the parent card effort copy when Light is selected", () => {
    selectToggle("Team preset", "light");
    const text = visibleText(renderGenerator());

    expect(text).toContain("Orchestrator");
    expect(text).toContain("Host managed");
    expect(text).toContain("Set Codex reasoning to Low.");
    expect(text).not.toContain("Not included");
    expect(text).not.toContain("Set Codex reasoning to Medium.");
  });

  it("updates the parent card effort copy when High is selected after Light", () => {
    selectToggle("Team preset", "light");
    expect(visibleText(renderGenerator())).toContain("Set Codex reasoning to Low.");

    selectToggle("Team preset", "high");
    const text = visibleText(renderGenerator());

    expect(text).toContain("Orchestrator");
    expect(text).toContain("Host managed");
    expect(text).toContain("Set Codex reasoning to Medium.");
    expect(text).not.toContain("Not included");
    expect(text).not.toContain("Set Codex reasoning to Low.");
  });

  it("does not render the parent card for Pi", () => {
    selectToggle("AI agent", "pi");
    const text = visibleText(renderGenerator());

    expect(text).toContain("4 focused roles");
    expect(text).toContain("Orchestrator");
    expect(text).toContain("openai/gpt-4o-mini · medium");
    expect(text).not.toContain("host-managed parent");
    expect(text).not.toContain("Host managed");
    expect(text).not.toContain("Not included");
    expect(text).not.toContain("Set Codex reasoning");
  });
});

describe("Generator connected child cards", () => {
  beforeEach(() => {
    reactState.reset();
  });

  it("renders every selected child with visible model and reasoning controls", () => {
    const tree = renderGenerator();
    const text = visibleText(tree);

    expect(text).toContain("Implementer");
    expect(text).toContain("Writes code and runs focused tests.");
    expect(text).toContain("Reviewer");
    expect(text).toContain("Finds defects independently.");
    expect(text).toContain("Verifier");
    expect(text).toContain("Proves the result actually works.");
    expect(findByAriaLabelOrNull(tree, "Edit Implementer")).toBeNull();
    expect(findByAriaLabel(tree, "Remove Reviewer").props.disabled).toBe(false);
    expect(findByAriaLabel(tree, "Implementer model")).toBeDefined();
    expect(findByAriaLabel(tree, "Implementer Reasoning")).toBeDefined();
    expect(findByAriaLabel(tree, "Reviewer model")).toBeDefined();
    expect(findByAriaLabel(tree, "Reviewer Reasoning")).toBeDefined();
    expect(findByAriaLabel(tree, "Verifier model")).toBeDefined();
    expect(findByAriaLabel(tree, "Verifier Reasoning")).toBeDefined();
  });

  it("removes a child card from its trash action and marks the preset custom", () => {
    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    const text = visibleText(renderGenerator());
    expect(text).not.toContain("Reviewer");
    expect(text).toContain("Implementer");
    expect(text).toContain("Verifier");
    expect(text).toContain("Custom");
  });

  it("keeps all child controls visible without pencil actions", () => {
    const tree = renderGenerator();
    expect(findByAriaLabelOrNull(tree, "Edit Implementer")).toBeNull();
    expect(findByAriaLabelOrNull(tree, "Edit Reviewer")).toBeNull();
    expect(findByAriaLabelOrNull(tree, "Edit Verifier")).toBeNull();
    expect(findByAriaLabel(tree, "Implementer model")).toBeDefined();
    expect(findByAriaLabel(tree, "Implementer Reasoning")).toBeDefined();
    expect(findByAriaLabel(tree, "Reviewer model")).toBeDefined();
    expect(findByAriaLabel(tree, "Reviewer Reasoning")).toBeDefined();
    expect(findByAriaLabel(tree, "Verifier model")).toBeDefined();
    expect(findByAriaLabel(tree, "Verifier Reasoning")).toBeDefined();
  });

  it("explains Codex Max as a manual single-agent option", () => {
    const reasoning = findByAriaLabel(renderGenerator(), "Reviewer Reasoning");
    const onValueChange = reasoning.props.onValueChange;
    if (typeof onValueChange !== "function") {
      throw new Error("Reviewer reasoning is not selectable");
    }
    onValueChange("max");

    expect(findByAriaLabel(renderGenerator(), "Reviewer Reasoning").props.valueNote)
      .toMatch(/single-agent|one Codex agent/i);
    expect(findByAriaLabel(renderGenerator(), "Reviewer Reasoning").props.valueNote)
      .toMatch(/app settings/i);
  });

  it("disables reset roles until the selected roles differ from the preset baseline", () => {
    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(true);

    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(false);
    expect(visibleText(renderGenerator())).toContain("Custom");
  });

  it("restores the last selected preset from the reset confirmation", () => {
    selectToggle("Team preset", "high");
    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();
    expect(visibleText(renderGenerator())).toContain("Custom");
    expect(visibleText(renderGenerator())).not.toContain("Reviewer");

    const confirmReset = findByAriaLabel(renderGenerator(), "Confirm reset roles").props.onClick;
    if (typeof confirmReset !== "function") {
      throw new Error("Reset confirmation is not clickable");
    }
    confirmReset();

    const text = visibleText(renderGenerator());
    expect(text).toContain("Reviewer");
    expect(text).toContain("High");
    expect(text).not.toContain("Custom");
    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(true);
  });

  it("keeps removed roles custom after host switching until reset restores the new host preset", () => {
    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    selectToggle("AI agent", "pi");

    const switchedText = visibleText(renderGenerator());
    expect(switchedText).toContain("Custom");
    expect(switchedText).not.toContain("Reviewer");
    expect(switchedText).toContain("openai/gpt-4o-mini · medium");
    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(false);

    const confirmReset = findByAriaLabel(renderGenerator(), "Confirm reset roles").props.onClick;
    if (typeof confirmReset !== "function") {
      throw new Error("Reset confirmation is not clickable");
    }
    confirmReset();

    const resetText = visibleText(renderGenerator());
    expect(resetText).toContain("Reviewer");
    expect(resetText).not.toContain("Custom");
    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(true);
  });

  it("renders reset confirmation with the project dialog treatment", () => {
    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    const resetDialogContent = findElement(renderGenerator(), (element) => {
      return typeof element.type === "function"
        && element.type.name === "DialogContent"
        && visibleText(element.props.children as ReactNode).includes("Reset roles?");
    });
    expect(resetDialogContent?.props.className).toContain("p-0");
    expect(resetDialogContent?.props.className).toContain("sm:max-w-lg");

    const resetDialogHeader = findElement(resetDialogContent, (element) => {
      return typeof element.type === "function"
        && element.type.name === "DialogHeader"
        && visibleText(element.props.children as ReactNode).includes("Destructive action");
    });
    expect(resetDialogHeader?.props.className).toContain("border-b");
    expect(resetDialogHeader?.props.className).toContain("px-5");
    expect(visibleText(resetDialogContent)).toContain("Cancel keeps your current role edits.");
  });

  it("keeps custom edits when reset confirmation is dismissed", () => {
    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    const resetDialog = findElement(renderGenerator(), (element) => {
      return typeof element.type === "function"
        && element.type.name === "Dialog"
        && element.props.open === false
        && typeof element.props.onOpenChange === "function";
    });
    const onOpenChange = resetDialog?.props.onOpenChange;
    if (typeof onOpenChange !== "function") {
      throw new Error("Reset confirmation dialog is not dismissible");
    }
    onOpenChange(false);

    const text = visibleText(renderGenerator());
    expect(text).toContain("Custom");
    expect(text).not.toContain("Reviewer");
    expect(findByAriaLabel(renderGenerator(), "Reset roles").props.disabled).toBe(false);
  });

  it("guards the final child role with a disabled remove affordance", () => {
    const removeImplementer = findByAriaLabel(renderGenerator(), "Remove Implementer").props.onClick;
    if (typeof removeImplementer !== "function") {
      throw new Error("Implementer remove action is not clickable");
    }
    removeImplementer();

    const removeReviewer = findByAriaLabel(renderGenerator(), "Remove Reviewer").props.onClick;
    if (typeof removeReviewer !== "function") {
      throw new Error("Reviewer remove action is not clickable");
    }
    removeReviewer();

    const tree = renderGenerator();
    expect(visibleText(tree)).toContain("At least one child role is required.");
    expect(findByAriaLabel(tree, "Cannot remove Verifier; at least one child role is required").props.disabled).toBe(true);
  });
});
