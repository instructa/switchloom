#!/usr/bin/env node
import { createServer } from "node:net";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";

const args = new Map();
for (let index = 2; index < process.argv.length; index += 2) {
  args.set(process.argv[index], process.argv[index + 1]);
}

const baseUrl = args.get("--base-url") ?? "http://127.0.0.1:4173";
const outDir = args.get("--out-dir") ?? "retained-evidence/live-web/generator";
const chromePath = args.get("--chrome") ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

async function freePort() {
  return await new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
  });
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchJson(url, options) {
  const response = await fetch(url, options);
  if (!response.ok) throw new Error(`${options?.method ?? "GET"} ${url} failed with ${response.status}`);
  return await response.json();
}

async function waitForJson(url, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      return await fetchJson(url);
    } catch (error) {
      lastError = error;
      await sleep(100);
    }
  }
  throw lastError ?? new Error(`timed out waiting for ${url}`);
}

function createCdpClient(webSocketUrl) {
  const socket = new WebSocket(webSocketUrl);
  let nextId = 1;
  const pending = new Map();
  const eventWaiters = new Map();

  socket.addEventListener("message", (message) => {
    const packet = JSON.parse(message.data);
    if (packet.id && pending.has(packet.id)) {
      const { resolve, reject } = pending.get(packet.id);
      pending.delete(packet.id);
      if (packet.error) reject(new Error(packet.error.message));
      else resolve(packet.result);
      return;
    }
    const waiters = eventWaiters.get(packet.method);
    if (waiters?.length) {
      waiters.splice(0).forEach((resolve) => resolve(packet.params));
    }
  });

  const opened = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  return {
    async send(method, params = {}) {
      await opened;
      const id = nextId;
      nextId += 1;
      const done = new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
      socket.send(JSON.stringify({ id, method, params }));
      return await done;
    },
    async waitFor(method) {
      await opened;
      return await new Promise((resolve) => {
        const waiters = eventWaiters.get(method) ?? [];
        waiters.push(resolve);
        eventWaiters.set(method, waiters);
      });
    },
    async close() {
      await opened;
      socket.close();
    },
  };
}

async function evaluate(client, expression) {
  const result = await client.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.exception?.description ?? "Runtime.evaluate failed");
  }
  return result.result.value;
}

const helpers = String.raw`
(() => {
  const text = (node = document.body) => (node?.innerText || node?.textContent || "").replace(/\s+/g, " ").trim();
  const visible = (element) => !!element && element.offsetParent !== null;
  const byLabel = (label) => [...document.querySelectorAll("button, input, [aria-label]")]
    .find((element) => element.getAttribute("aria-label") === label);
  const byText = (wanted) => [...document.querySelectorAll("button, [role=button]")]
    .find((element) => visible(element) && text(element) === wanted);
  const clickLabel = (label) => {
    const element = byLabel(label);
    if (!element) throw new Error("missing labelled control: " + label);
    element.click();
  };
  const clickText = (wanted) => {
    const element = byText(wanted);
    if (!element) throw new Error("missing text control: " + wanted);
    element.click();
  };
  const focusLabel = (label) => {
    const element = byLabel(label);
    if (!element) throw new Error("missing focus target: " + label);
    element.focus();
    return document.activeElement === element;
  };
  const spec = () => {
    const tab = byText("Spec");
    if (tab) tab.click();
    return text(document.querySelector("pre"));
  };
  const has = (value) => text().includes(value);
  const not = (value) => !text().includes(value);
  return { text: text(), has, not, clickLabel, clickText, focusLabel, activeLabel: document.activeElement?.getAttribute("aria-label") || "", spec };
})()
`;

async function pageState(client) {
  return await evaluate(client, "document.body.innerText");
}

async function waitFor(client, predicateSource, message, timeoutMs = 8000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await evaluate(client, `Boolean((${predicateSource})())`)) return;
    await sleep(100);
  }
  throw new Error(message);
}

async function clickLabel(client, label) {
  await evaluate(client, `${helpers}.clickLabel(${JSON.stringify(label)})`);
  await sleep(150);
}

async function clickText(client, text) {
  await evaluate(client, `${helpers}.clickText(${JSON.stringify(text)})`);
  await sleep(150);
}

async function focusLabel(client, label) {
  const focused = await evaluate(client, `${helpers}.focusLabel(${JSON.stringify(label)})`);
  assert(focused, `failed to focus ${label}`);
}

async function scrollTeamHierarchyIntoView(client) {
  await evaluate(client, String.raw`
(() => {
  const legends = [...document.querySelectorAll("legend")];
  const legend = legends.find((element) => element.textContent?.includes("3. Tune each role"));
  if (!legend) throw new Error("team hierarchy legend missing");
  legend.scrollIntoView({ block: "start", inline: "nearest" });
})()
`);
  await sleep(150);
}

async function assertNoHorizontalOverflow(client, label) {
  const overflow = JSON.parse(await evaluate(client, String.raw`
JSON.stringify((() => ({
  viewportWidth: window.innerWidth,
  documentWidth: document.documentElement.scrollWidth,
  bodyWidth: document.body.scrollWidth,
}))())
`));
  assert(
    overflow.documentWidth <= overflow.viewportWidth + 1 && overflow.bodyWidth <= overflow.viewportWidth + 1,
    `${label}: horizontal overflow detected: viewport ${overflow.viewportWidth}, document ${overflow.documentWidth}, body ${overflow.bodyWidth}`,
  );
}

async function assertHierarchyGeometry(client, label) {
  const geometry = JSON.parse(await evaluate(client, String.raw`
JSON.stringify((() => {
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return { top: box.top, right: box.right, bottom: box.bottom, left: box.left, width: box.width, height: box.height };
  };
  const intersects = (a, b) => !(a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top);
  const wrappers = [...document.querySelectorAll("div")]
    .filter((element) => element.classList.contains("relative") && element.classList.contains("pl-6"));
  const rows = wrappers.map((wrapper, index) => {
    const card = wrapper.querySelector('[data-slot="card"]');
    const connector = wrapper.querySelector('span[aria-hidden="true"]');
    const header = card?.querySelector('[data-slot="card-header"]');
    const titleColumn = header?.firstElementChild;
    const action = header?.querySelector('[data-slot="card-action"]');
    const wrapperRect = rect(wrapper);
    const cardRect = card ? rect(card) : null;
    const connectorRect = connector ? rect(connector) : null;
    const headerRect = header ? rect(header) : null;
    const titleRect = titleColumn ? rect(titleColumn) : null;
    const actionRect = action ? rect(action) : null;
    const before = getComputedStyle(wrapper, "::before");
    return {
      index,
      wrapperRect,
      cardRect,
      connectorRect,
      headerRect,
      titleRect,
      actionRect,
      beforeWidth: Number.parseFloat(before.width),
      beforeHeight: Number.parseFloat(before.height),
      beforeDisplay: before.display,
      actionOverlapsText: Boolean(titleRect && actionRect && intersects(titleRect, actionRect)),
    };
  });
  return {
    viewportWidth: window.innerWidth,
    rows,
  };
})())
`));

  assert(geometry.rows.length === 3, `${label}: expected 3 connected child cards, found ${geometry.rows.length}`);
  for (const row of geometry.rows) {
    assert(row.cardRect, `${label}: child ${row.index + 1} card missing`);
    assert(row.connectorRect, `${label}: child ${row.index + 1} connector missing`);
    assert(row.beforeDisplay !== "none" && row.beforeWidth >= 1 && row.beforeHeight >= 20, `${label}: child ${row.index + 1} vertical connector is not visible`);
    assert(row.connectorRect.width >= 12 && row.connectorRect.height >= 1, `${label}: child ${row.index + 1} horizontal connector is not visible`);
    assert(row.connectorRect.left >= row.wrapperRect.left, `${label}: child ${row.index + 1} connector escapes wrapper`);
    assert(row.connectorRect.right <= row.cardRect.left + 1, `${label}: child ${row.index + 1} connector is not aligned to card edge`);
    assert(row.connectorRect.top >= row.cardRect.top && row.connectorRect.top <= row.cardRect.bottom, `${label}: child ${row.index + 1} connector misses card height`);
    assert(row.cardRect.right <= geometry.viewportWidth + 1, `${label}: child ${row.index + 1} card overflows viewport`);
    assert(row.actionRect, `${label}: child ${row.index + 1} action controls missing`);
    assert(row.headerRect && row.actionRect.right <= row.headerRect.right + 1, `${label}: child ${row.index + 1} action controls overflow header`);
    assert(!row.actionOverlapsText, `${label}: child ${row.index + 1} action controls overlap title text`);
  }
}

async function assertResetDialogGeometry(client, label) {
  const geometry = JSON.parse(await evaluate(client, String.raw`
JSON.stringify((() => {
  const rect = (element) => {
    const box = element.getBoundingClientRect();
    return { top: box.top, right: box.right, bottom: box.bottom, left: box.left, width: box.width, height: box.height };
  };
  const dialog = document.querySelector('[data-slot="dialog-content"]');
  const footer = dialog?.querySelector('[data-slot="dialog-footer"]');
  const buttons = footer ? [...footer.querySelectorAll("button")].map(rect) : [];
  return {
    viewportWidth: window.innerWidth,
    dialog: dialog ? rect(dialog) : null,
    buttons,
    text: dialog?.innerText || "",
  };
})())
`));
  assert(geometry.dialog, `${label}: reset dialog is not visible`);
  assert(geometry.text.includes("Reset roles?"), `${label}: reset dialog title missing`);
  assert(geometry.dialog.left >= 0 && geometry.dialog.right <= geometry.viewportWidth + 1, `${label}: reset dialog overflows viewport`);
  for (const [index, button] of geometry.buttons.entries()) {
    assert(button.left >= geometry.dialog.left && button.right <= geometry.dialog.right + 1, `${label}: dialog button ${index + 1} overflows dialog`);
  }
}

async function capture(client, fileName) {
  const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
  const path = join(outDir, fileName);
  await writeFile(path, Buffer.from(screenshot.data, "base64"));
  return path;
}

async function setViewport(client, width, height, scale = 1) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width,
    height,
    deviceScaleFactor: scale,
    mobile: width < 600,
  });
}

async function main() {
  await mkdir(outDir, { recursive: true });
  const port = await freePort();
  const profile = join(tmpdir(), `switchloom-live-web-${process.pid}`);
  const chrome = spawn(chromePath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    "about:blank",
  ], { stdio: ["ignore", "ignore", "pipe"] });

  let stderr = "";
  chrome.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
  });

  try {
    await waitForJson(`http://127.0.0.1:${port}/json/version`);
    const target = await fetchJson(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(baseUrl)}`, { method: "PUT" });
    const client = createCdpClient(target.webSocketDebuggerUrl);
    await client.send("Runtime.enable");
    await client.send("Page.enable");

    await setViewport(client, 1440, 1000);
    await client.send("Page.navigate", { url: baseUrl });
    await client.waitFor("Page.loadEventFired");
    await waitFor(client, "() => document.body.innerText.includes('Build your coding-agent team.')", "generator did not render");

    let state = await pageState(client);
    assert(state.includes("Host managed"), "desktop: host-managed parent copy missing");
    assert(state.includes("3 generated child roles"), "desktop: generated child count missing");
    assert(state.includes("Set Codex reasoning to Medium."), "desktop: balanced parent effort copy missing");
    assert(!state.includes("Not included"), "desktop: obsolete parent copy is still visible");
    const desktopShot = await capture(client, "desktop-generator.png");
    await scrollTeamHierarchyIntoView(client);
    await assertNoHorizontalOverflow(client, "desktop hierarchy");
    await assertHierarchyGeometry(client, "desktop hierarchy");
    const desktopHierarchyShot = await capture(client, "desktop-generator-hierarchy.png");

    await focusLabel(client, "Edit Implementer");
    await client.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: " ", code: "Space", windowsVirtualKeyCode: 32 });
    await client.send("Input.dispatchKeyEvent", { type: "keyUp", key: " ", code: "Space", windowsVirtualKeyCode: 32 });
    await waitFor(client, "() => document.body.innerText.includes('Model') && document.querySelector('[aria-label=\"Implementer model\"]')", "keyboard edit did not expose Implementer controls");

    await clickLabel(client, "Remove Reviewer");
    state = await pageState(client);
    assert(state.includes("Custom"), "removal: custom preset badge missing");
    assert(state.includes("2 generated child roles"), "removal: child count did not update");
    assert(!state.includes("Reviewer Finds defects independently."), "removal: reviewer card still visible");

    await clickLabel(client, "Reset roles");
    await waitFor(client, "() => document.body.innerText.includes('Reset roles?')", "reset dialog did not open");
    await assertResetDialogGeometry(client, "desktop reset dialog");
    const desktopResetShot = await capture(client, "desktop-reset-confirmation.png");
    await clickText(client, "Cancel");
    state = await pageState(client);
    assert(state.includes("Custom"), "cancel reset: custom state was lost");
    assert(state.includes("2 generated child roles"), "cancel reset: removed role came back");

    await clickLabel(client, "Reset roles");
    await clickLabel(client, "Confirm reset roles");
    await waitFor(client, "() => document.body.innerText.includes('3 generated child roles') && !document.body.innerText.includes('Custom')", "confirm reset did not restore preset");

    await clickText(client, "Pi");
    await waitFor(client, "() => document.body.innerText.includes('Pi team')", "host switch to Pi failed");
    state = await pageState(client);
    assert(state.includes("4 focused roles"), "host switch: Pi should render all focused roles");
    assert(!state.includes("host-managed parent"), "host switch: Pi should not show native parent summary");
    await clickText(client, "Spec");
    await waitFor(client, "() => Boolean(document.querySelector('pre')?.textContent)", "spec tab did not render JSON");
    const piSpec = await evaluate(client, "document.querySelector('pre')?.textContent || ''");
    assert(piSpec.includes('"orchestrator"'), "host switch: Pi spec should include orchestrator");

    await clickText(client, "Codex");
    await clickText(client, "Light");
    await waitFor(client, "() => document.body.innerText.includes('Set Codex reasoning to Low.')", "light preset effort copy missing");
    await clickText(client, "High");
    await waitFor(client, "() => document.body.innerText.includes('Set Codex reasoning to Medium.')", "high preset effort copy missing");

    await setViewport(client, 390, 844, 2);
    await client.send("Page.reload", { ignoreCache: true });
    await client.waitFor("Page.loadEventFired");
    await waitFor(client, "() => document.body.innerText.includes('Build your coding-agent team.') && document.body.innerText.includes('Host managed')", "mobile: generator did not render");
    state = await pageState(client);
    assert(state.includes("3 generated child roles"), "mobile: generated child count missing");
    assert(state.includes("Reset roles"), "mobile: reset control missing");
    await scrollTeamHierarchyIntoView(client);
    await assertNoHorizontalOverflow(client, "mobile hierarchy");
    await assertHierarchyGeometry(client, "mobile hierarchy");
    const mobileShot = await capture(client, "mobile-generator.png");
    await clickLabel(client, "Remove Reviewer");
    await clickLabel(client, "Reset roles");
    await waitFor(client, "() => document.body.innerText.includes('Reset roles?')", "mobile: reset dialog did not open");
    await assertResetDialogGeometry(client, "mobile reset dialog");
    const mobileResetShot = await capture(client, "mobile-reset-confirmation.png");

    await client.close();
    console.log(JSON.stringify({
      ok: true,
      baseUrl,
      screenshots: [desktopShot, desktopHierarchyShot, desktopResetShot, mobileShot, mobileResetShot],
      checks: [
        "desktop host-managed parent and generated child count",
        "desktop hierarchy connectors and action-control geometry",
        "desktop reset confirmation capture",
        "keyboard activation of Edit Implementer",
        "remove Reviewer",
        "cancel reset preserves custom edits",
        "confirm reset restores selected preset",
        "host switch to Pi includes orchestrator in spec",
        "preset switching updates Codex parent effort copy",
        "narrow mobile render at 390x844",
        "narrow mobile hierarchy connectors, action-control geometry, and horizontal overflow",
        "narrow mobile reset confirmation capture",
      ],
    }, null, 2));
  } finally {
    chrome.kill("SIGTERM");
    await rm(profile, { recursive: true, force: true });
    if (chrome.exitCode && chrome.exitCode !== 0) {
      process.stderr.write(stderr);
    }
  }
}

main().catch((error) => {
  console.error(error.stack || error.message);
  process.exit(1);
});
