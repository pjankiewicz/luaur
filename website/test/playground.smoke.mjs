// Playground smoke test — boots the real static site in headless Chromium and
// asserts the *typed* run/error behavior end-to-end (wasm engine + panic-hook
// bridge + app.js classification). No test framework: a tiny static server, a
// handful of assertions, exit non-zero on any failure.
//
//   cd website/test && npm install && npx playwright install chromium && npm test
//
// Prereq: website/pkg must hold a fresh wasm build (see ../../README or the
// release checklist). The test serves website/ as-is.

import { chromium } from "playwright";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url)); // website/
const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".luau": "text/plain; charset=utf-8",
  ".lua": "text/plain; charset=utf-8",
};

// ── tiny static file server over website/ ──────────────────────────────────
function startServer() {
  const server = http.createServer(async (req, res) => {
    try {
      const url = new URL(req.url, "http://localhost");
      let p = normalize(decodeURIComponent(url.pathname));
      if (p === "/" || p === "") p = "/index.html";
      if (p.includes("..")) {
        res.writeHead(403).end("forbidden");
        return;
      }
      const body = await readFile(join(ROOT, p));
      res.writeHead(200, { "content-type": MIME[extname(p)] || "application/octet-stream" });
      res.end(body);
    } catch {
      res.writeHead(404).end("not found");
    }
  });
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => resolve({ server, port: server.address().port }));
  });
}

// ── assertions ─────────────────────────────────────────────────────────────
const failures = [];
function check(name, cond, detail) {
  if (cond) {
    console.log(`  ok  ${name}`);
  } else {
    console.log(`FAIL  ${name}  — ${detail ?? ""}`);
    failures.push(name);
  }
}

async function main() {
  const { server, port } = await startServer();
  const base = `http://127.0.0.1:${port}`;
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  await page.goto(`${base}/index.html`, { waitUntil: "load" });

  // ── 1. engine layer: typed {output, error}, fresh module per case ─────────
  // The reported bug: iterating _G prints a global literally named "error";
  // classification must come from the typed `error` field, never the text.
  const eng = await page.evaluate(async () => {
    // Borrow the runtime-error bridge for this phase, then restore app.js's own
    // handler so the later UI checks still see their messages.
    const appHandler = globalThis.__luaurOnRuntimeError;
    let lastErr = "";
    globalThis.__luaurOnRuntimeError = (m) => { lastErr = String(m ?? ""); };
    const out = {};
    const fresh = async (tag) => { const m = await import(`./pkg/luaur_web.js?smoke=${tag}`); await m.default(); return m; };

    let m = await fresh("g");
    { const r = m.run('print(_G)\nfor i, v in _G do print(i) end\nprint("Done")');
      out.gG = { errorEmpty: r.error === "", endsDone: r.output.trim().endsWith("Done") }; r.free(); }
    { const r = m.run('local x ='); out.compile = { errorNonEmpty: r.error.length > 0 }; r.free(); }

    m = await fresh("rt"); lastErr = "";
    { let isRT = false; try { m.run('error("kaboom-smoke")'); } catch (e) { isRT = e instanceof WebAssembly.RuntimeError; }
      out.runtime = { isWasmRuntimeError: isRT, msg: lastErr }; }
    globalThis.__luaurOnRuntimeError = appHandler; // restore app.js's bridge
    return out;
  });
  check("engine: _G run has empty (typed) error", eng.gG.errorEmpty);
  check("engine: _G output ends with Done", eng.gG.endsDone);
  check("engine: compile error sets the error field", eng.compile.errorNonEmpty);
  check("engine: runtime error traps as WebAssembly.RuntimeError", eng.runtime.isWasmRuntimeError);
  check("engine: runtime error message bridged to JS", eng.runtime.msg.includes("kaboom-smoke"), `got: ${eng.runtime.msg}`);

  // ── 2. real UI: the globals example must classify OK (out-ok), not red ────
  const ui1 = await page.evaluate(async () => {
    const sel = document.getElementById("example-select");
    sel.value = "globals"; sel.dispatchEvent(new Event("change"));
    await new Promise((r) => setTimeout(r, 80));
    document.getElementById("btn-run").click();
    await new Promise((r) => setTimeout(r, 500));
    const span = document.querySelector("#output span");
    return { cls: span?.className, text: (document.getElementById("output").textContent || "").trim() };
  });
  check("ui: globals example is present + classified out-ok", ui1.cls === "out-ok", `class=${ui1.cls}`);
  check("ui: globals output ends with Done", ui1.text.endsWith("Done"));

  // ── 3. real UI: a runtime error → out-err with the bridged message ────────
  await page.locator(".cm-content").click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.locator(".cm-content").fill('error("ui-smoke-runtime")');
  const ui2 = await page.evaluate(async () => {
    document.getElementById("btn-run").click();
    await new Promise((r) => setTimeout(r, 700));
    const span = document.querySelector("#output span");
    return { cls: span?.className, text: span?.textContent || "", status: document.getElementById("status")?.textContent };
  });
  check("ui: runtime error classified out-err", ui2.cls === "out-err", `class=${ui2.cls}`);
  check("ui: runtime error shows the bridged message", ui2.text.includes("ui-smoke-runtime"), `got: ${ui2.text}`);
  check("ui: status reads 'runtime error'", ui2.status === "runtime error", `got: ${ui2.status}`);

  // ── 4. real UI: engine recovers after the trap (next run works) ───────────
  const ui3 = await page.evaluate(async () => {
    const sel = document.getElementById("example-select");
    sel.value = "hello"; sel.dispatchEvent(new Event("change"));
    await new Promise((r) => setTimeout(r, 100));
    document.getElementById("btn-run").click();
    await new Promise((r) => setTimeout(r, 600));
    const span = document.querySelector("#output span");
    return { cls: span?.className, first: (span?.textContent || "").split("\n")[0] };
  });
  check("ui: engine recovers after a trap (next run is out-ok)", ui3.cls === "out-ok", `class=${ui3.cls}`);
  check("ui: recovered run produces output", ui3.first.length > 0);

  check("no uncaught page errors", pageErrors.length === 0, pageErrors.join(" | "));

  await browser.close();
  server.close();

  if (failures.length) {
    console.error(`\n${failures.length} check(s) failed: ${failures.join(", ")}`);
    process.exit(1);
  }
  console.log("\nAll playground smoke checks passed.");
}

main().catch((e) => { console.error(e); process.exit(1); });
