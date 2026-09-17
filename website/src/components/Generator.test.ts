import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import Generator from "./Generator";
import catalog from "../../data/catalog.json";
it("renders the board with accessible reset, model toggles and move controls", () => {
  const markup = renderToStaticMarkup(createElement(Generator, { workflow: catalog }));
  expect(markup).toContain('aria-label="Reset to default"');
  expect(markup).toContain('aria-label="Enable GPT-5.6 Luna"');
  expect(markup).toContain('aria-label="Move Browser use"');
  expect(markup).toContain('aria-label="Workflow prompt"');
  expect(markup.indexOf('aria-label="luna model"')).toBeLessThan(markup.indexOf('aria-label="sol model"'));
  expect(markup).not.toMatch(/Advanced|Drag to assign|Add coordinator/);
});
