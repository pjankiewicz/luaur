// luaur playground — boots the WebAssembly engine and wires the editor + buttons.
//
// The Luau compiler, VM and type checker are compiled to wasm (crates/luaur-web,
// `wasm` feature) and exposed as two functions:
//   run(source)   -> captured print output + any runtime error
//   check(source) -> type-checker diagnostics, or "No errors."
//
// CodeMirror 6 (with the legacy Lua mode) provides the editor. Everything runs
// client-side; there is no server.

import { setMemory } from "./env.js";

// The wasm module is loaded dynamically (rather than a static import) so it can
// be RE-INSTANTIATED after a wasm trap. The faithful translation emulates Lua's
// C++ exceptions with Rust `panic_any` + `catch_unwind` (parser ParseError, VM
// runtime errors / `error()` / failed `pcall`). On wasm32-unknown-unknown the
// stable std only supports `panic = "abort"`, so those recoverable panics abort
// the instance instead of unwinding. We keep the standard stable build and make
// the playground resilient: a trapped call is caught in JS and the engine is
// transparently reloaded, so the playground never gets stuck. (A successful run
// or type-check returns normally and needs no reload.)
let engine = null; // { run, check }
let engineGen = 0; // cache-buster so each reload gets a fresh module instance

async function loadEngine() {
  engineGen += 1;
  const mod = await import(`./pkg/luaur_web.js?gen=${engineGen}`);
  const wasm = await mod.default();
  setMemory(wasm.memory);
  engine = { run: mod.run, check: mod.check };
}

// CodeMirror 6 is vendored as a single self-contained ESM bundle
// (codemirror.bundle.js). Loading the CM packages individually from a CDN
// pulls in multiple copies of @codemirror/state and breaks CodeMirror's
// internal `instanceof` checks ("Unrecognized extension value"); one local
// bundle guarantees a single shared @codemirror/state instance and works
// offline with no CDN version-matrix fragility.
import {
  EditorView,
  EditorState,
  basicSetup,
  keymap,
  StreamLanguage,
  lua,
  oneDark,
} from "./codemirror.bundle.js";

// ─────────────────────────── examples (inlined) ───────────────────────────
// Inlined so the page works as a pure static file (no fetch / CORS needed),
// even when opened over file://. These are the same scripts shipped in
// website/examples/*.luau.
const EXAMPLES = {
  hello: {
    label: "Hello, world (basics)",
    source: `-- hello.luau
-- The classic first program: print a greeting, then show a few basics.

-- \`print\` accepts multiple arguments, separated by tabs in the output.
print("Hello, world!")

-- Numbers are double-precision; arithmetic works as expected.
local a = 7
local b = 5
print("sum:", a + b)          -- 12
print("product:", a * b)      -- 35
print("power:", a ^ 2)        -- 49 (^ is exponentiation)
print("remainder:", a % b)    -- 2  (% is modulo)

-- Strings are joined with the \`..\` concatenation operator.
local name = "Luau"
print("Welcome to " .. name .. "!")

-- Numbers are coerced to strings inside concatenation.
print("a + b = " .. (a + b))

-- \`print\` with several values of mixed types on one line.
print("mixed:", 42, true, "text", nil)
`,
  },
  fibonacci: {
    label: "Fibonacci (recursion)",
    source: `-- fibonacci.luau
-- Two ways to compute Fibonacci numbers: recursion and iteration.

-- Recursive definition: fib(0) = 0, fib(1) = 1, fib(n) = fib(n-1) + fib(n-2).
local function fibRecursive(n)
\tif n < 2 then
\t\treturn n
\tend
\treturn fibRecursive(n - 1) + fibRecursive(n - 2)
end

-- Iterative definition: keep the last two values and step forward.
local function fibIterative(n)
\tlocal prev, curr = 0, 1
\tfor _ = 1, n do
\t\tprev, curr = curr, prev + curr
\tend
\treturn prev
end

print("Recursive:")
for i = 0, 9 do
\tprint(i, fibRecursive(i))
end

print("Iterative:")
for i = 0, 9 do
\tprint(i, fibIterative(i))
end
`,
  },
  tables: {
    label: "Tables (arrays + dictionaries)",
    source: `-- tables.luau
-- Tables are Luau's single data structure: they act as arrays AND dictionaries.

-- An array-like table: consecutive integer keys starting at 1.
local fruits = { "apple", "banana", "cherry" }

-- \`#t\` gives the length (number of array entries).
print("number of fruits:", #fruits)

-- \`ipairs\` walks array entries in order (1, 2, 3, ...).
print("fruits in order:")
for index, fruit in ipairs(fruits) do
\tprint(index, fruit)
end

-- \`table.insert\` appends to the end of the array part.
table.insert(fruits, "date")
print("after insert, length:", #fruits)

-- A dictionary-like table: arbitrary string keys mapping to values.
local ages = { alice = 30, bob = 25, carol = 41 }

-- \`pairs\` walks every key/value pair.
print("ages:")
for name, age in pairs(ages) do
\tprint(name, age)
end

-- Tables can mix and nest freely.
local person = { name = "Dave", hobbies = { "chess", "cycling" } }
print(person.name .. " enjoys " .. person.hobbies[1] .. " and " .. person.hobbies[2])
`,
  },
  metatables: {
    label: "Metatables (OOP + operators)",
    source: `-- metatables.luau
-- Object-oriented programming in Luau is built on metatables.
-- Here we make a small Vector2 "class".

local Vector2 = {}
-- \`__index = Vector2\` means: missing fields on an instance fall back to Vector2.
Vector2.__index = Vector2

-- Constructor.
function Vector2.new(x, y)
\tlocal self = setmetatable({}, Vector2)
\tself.x = x
\tself.y = y
\treturn self
end

-- A method. \`self\` is the instance (via the \`:\` call syntax).
function Vector2:magnitude()
\treturn (self.x * self.x + self.y * self.y) ^ 0.5
end

-- Operator overloading: \`__add\` is called for the \`+\` operator.
function Vector2.__add(a, b)
\treturn Vector2.new(a.x + b.x, a.y + b.y)
end

function Vector2:toString()
\treturn "(" .. self.x .. ", " .. self.y .. ")"
end

local v1 = Vector2.new(3, 4)
local v2 = Vector2.new(1, 2)

print("v1:", v1:toString())
print("v2:", v2:toString())
print("v1 magnitude:", v1:magnitude())   -- 5

local sum = v1 + v2                        -- dispatches to __add
print("v1 + v2:", sum:toString())          -- (4, 6)
`,
  },
  strings: {
    label: "Strings (string library)",
    source: `-- strings.luau
-- A tour of the string library.

local s = "Hello, Luau"

print("length:", #s)                         -- byte length
print("upper:", string.upper(s))
print("lower:", string.lower(s))
print("sub(1, 5):", string.sub(s, 1, 5))     -- "Hello"
print("sub(8):", string.sub(s, 8))            -- "Luau"
print("rep:", string.rep("ab", 3))            -- "ababab"

-- string.format works like C printf.
print(string.format("name=%s pi=%.2f count=%d", "Luau", 3.14159, 42))

-- string.find returns the start and end indices of a match (or nil).
local start, finish = string.find(s, "Luau")
print("found 'Luau' at:", start, finish)      -- 8  11

-- gsub does a global substitution and returns the result plus a count.
local replaced, count = s:gsub("l", "L")
print("gsub result:", replaced)
print("gsub count:", count)
`,
  },
  coroutines: {
    label: "Coroutines (generators)",
    source: `-- coroutines.luau
-- Coroutines are cooperative, resumable functions: they pause with
-- \`coroutine.yield\` and continue with \`coroutine.resume\`.

local function producer()
\tcoroutine.yield("first")
\tcoroutine.yield("second")
\tcoroutine.yield("third")
\treturn "done"
end

local routine = coroutine.create(producer)

-- Each resume runs until the next yield (or return).
print("resume 1:", coroutine.resume(routine))   -- true  first
print("resume 2:", coroutine.resume(routine))   -- true  second
print("resume 3:", coroutine.resume(routine))   -- true  third
print("resume 4:", coroutine.resume(routine))   -- true  done
print("status:", coroutine.status(routine))      -- dead

-- coroutine.wrap turns a coroutine into a plain function.
local function squaresUpTo(n)
\treturn coroutine.wrap(function()
\t\tfor i = 1, n do
\t\t\tcoroutine.yield(i * i)
\t\tend
\tend)
end

print("squares:")
local nextSquare = squaresUpTo(5)
for _ = 1, 5 do
\tprint(nextSquare())
end
`,
  },
  typed: {
    label: "Typed (--!strict, type-checks clean)",
    source: `--!strict
-- typed.luau
-- Luau is gradually typed. With \`--!strict\`, the type checker verifies
-- annotations. This file is correct and type-checks cleanly.

-- A type alias names a shape so it can be reused.
type Point = { x: number, y: number }

local greeting: string = "Typed Luau"
local count: number = 3

-- A function with typed parameters and a typed return value.
local function distance(a: Point, b: Point): number
\tlocal dx: number = a.x - b.x
\tlocal dy: number = a.y - b.y
\treturn (dx * dx + dy * dy) ^ 0.5
end

local origin: Point = { x = 0, y = 0 }
local target: Point = { x = 3, y = 4 }

print(greeting)
print("count:", count)
print("distance:", distance(origin, target))   -- 5

-- An array typed as a list of numbers.
local scores: { number } = { 10, 20, 30 }
local total: number = 0
for _, score in ipairs(scores) do
\ttotal += score
end
print("total score:", total)   -- 60
`,
  },
  type_error: {
    label: "Type error (run Type-check!)",
    source: `--!strict
-- type_error.luau
-- This file is DELIBERATELY wrong. The syntax is valid (it parses), but the
-- types do not line up. Press "Type-check" to see the analyzer catch it.

-- A function that expects a number and returns a number.
local function double(n: number): number
\treturn n * 2
end

-- ERROR: \`count\` is declared as a number but assigned a string literal.
local count: number = "not a number"

-- ERROR: \`double\` expects a number argument, but we pass a string.
print(double("ten"))

print(count)
`,
  },
};

const DEFAULT_EXAMPLE = "hello";

// ─────────────────────────── DOM refs ───────────────────────────
const $ = (id) => document.getElementById(id);
const elOutput = $("output");
const elStatus = $("status");
const elRun = $("btn-run");
const elCheck = $("btn-check");
const elClear = $("btn-clear");
const elSelect = $("example-select");

let editor = null;
const AUTOCHECK_DEBOUNCE_MS = 450; // run check() this long after the last keystroke
let autoCheckTimer = null;
let autoCheckSuppressed = false; // paused briefly right after a Run shows output
let suppressTimer = null;

// ─────────────────────────── editor ───────────────────────────
function makeEditor(initialDoc) {
  const runShortcut = keymap.of([
    {
      key: "Mod-Enter",
      run: () => {
        doRun();
        return true;
      },
    },
  ]);

  // Re-run the (always-precise) type checker a short debounce after each edit.
  const autoCheckListener = EditorView.updateListener.of((update) => {
    if (update.docChanged) {
      autoCheckSuppressed = false; // editing re-enables live checking after a Run
      scheduleAutoCheck();
    }
  });

  const state = EditorState.create({
    doc: initialDoc,
    extensions: [
      basicSetup,
      StreamLanguage.define(lua),
      oneDark,
      runShortcut,
      autoCheckListener,
      EditorView.theme({
        "&": { height: "100%", backgroundColor: "transparent" },
        ".cm-scroller": { overflow: "auto" },
        ".cm-gutters": { backgroundColor: "transparent", border: "none" },
      }),
    ],
  });

  return new EditorView({ state, parent: $("editor") });
}

function getSource() {
  return editor ? editor.state.doc.toString() : "";
}

function setSource(text) {
  if (!editor) return;
  editor.dispatch({
    changes: { from: 0, to: editor.state.doc.length, insert: text },
  });
}

// ─────────────────────────── output helpers ───────────────────────────
function setStatus(text, kind) {
  elStatus.textContent = text;
  elStatus.className = "status status-" + kind;
}

function escapeHtml(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function writeOutput(text, kind) {
  const cls = kind ? ` class="${kind}"` : "";
  elOutput.classList.remove("is-booting");
  elOutput.innerHTML = `<span${cls}>${escapeHtml(text)}</span>`;
}

// Append a raw HTML fragment after the main output (used for the caveat block).
function appendOutputHtml(html) {
  elOutput.insertAdjacentHTML("beforeend", html);
}

// ─────────────────────────── diagnostics ───────────────────────────
// The wasm `check(source)` returns either "No errors." or newline-joined
// diagnostics, each shaped roughly "line: message" (and the analyzer can emit
// the *same* diagnostic more than once). This is the path that is fully
// precise on the stable wasm32-unknown-unknown build, so we render it as the
// primary signal in the output pane — clean, deduplicated, grouped by line.

// Parse the raw check() string into a sorted, de-duplicated list of
// { line:number|null, message:string }.
function parseDiagnostics(raw) {
  const seen = new Set();
  const out = [];
  for (const rawLine of raw.split("\n")) {
    const line = rawLine.trim();
    if (!line) continue;
    const key = line; // dedupe on the full "N: message"
    if (seen.has(key)) continue;
    seen.add(key);
    const m = line.match(/^(\d+)(?:\s*,\s*\d+)?\s*:\s*(.*)$/s);
    if (m) out.push({ line: parseInt(m[1], 10), message: m[2].trim() });
    else out.push({ line: null, message: line });
  }
  out.sort((a, b) => (a.line ?? Infinity) - (b.line ?? Infinity));
  return out;
}

// Render the diagnostics list as a code-aligned report in the output pane.
// `header` is the summary line shown above the rows.
function renderDiagnosticsToOutput(diags, kindClass) {
  elOutput.classList.remove("is-booting");
  const count = diags.length;
  const header =
    `<div class="diag-head ${kindClass}">` +
    `<span class="diag-mark">✗</span>` +
    `<span>${count} type ${count === 1 ? "issue" : "issues"}</span>` +
    `</div>`;
  const rows = diags
    .map((d) => {
      const ln =
        d.line != null
          ? `<a class="diag-ln" href="#" data-line="${d.line}" title="Jump to line ${d.line}">L${d.line}</a>`
          : `<span class="diag-ln diag-ln-none">—</span>`;
      return (
        `<div class="diag-row">` +
        ln +
        `<span class="diag-msg">${escapeHtml(d.message)}</span>` +
        `</div>`
      );
    })
    .join("");
  elOutput.innerHTML = `<div class="diag-report">${header}${rows}</div>`;
  wireDiagJumps();
}

function renderOkToOutput() {
  elOutput.classList.remove("is-booting");
  elOutput.innerHTML =
    `<div class="diag-report">` +
    `<div class="diag-head is-ok"><span class="diag-mark">✓</span>` +
    `<span>No type errors</span></div>` +
    `<div class="diag-okline">The analyzer accepts this program. Press ` +
    `<span class="kbd-inline">Run</span> to execute it on the Rust VM.</div>` +
    `</div>`;
}

// Clicking a line chip moves the editor cursor/selection to that line.
function wireDiagJumps() {
  for (const a of elOutput.querySelectorAll(".diag-ln[data-line]")) {
    a.addEventListener("click", (ev) => {
      ev.preventDefault();
      const n = parseInt(a.getAttribute("data-line"), 10);
      jumpToLine(n);
    });
  }
}

function jumpToLine(n) {
  if (!editor) return;
  const doc = editor.state.doc;
  if (n < 1 || n > doc.lines) return;
  const line = doc.line(n);
  editor.dispatch({
    selection: { anchor: line.from, head: line.to },
    scrollIntoView: true,
  });
  editor.focus();
}

// Returns true when there are errors. Drives both auto-check and the button.
function renderDiagnostics(result) {
  const trimmed = result.trim();
  if (trimmed === "No errors." || trimmed === "") {
    renderOkToOutput();
    setStatus("no type errors", "ready");
    return false;
  }
  const diags = parseDiagnostics(trimmed);
  if (diags.length === 0) {
    renderOkToOutput();
    setStatus("no type errors", "ready");
    return false;
  }
  renderDiagnosticsToOutput(diags, "is-err");
  setStatus(
    diags.length === 1 ? "1 type error" : diags.length + " type errors",
    "error"
  );
  return true;
}

// Run check() and paint the output pane. Trap-safe: a trapped checker reloads
// silently and we leave the previous output untouched.
async function autoCheck() {
  if (!engine || autoCheckSuppressed) return;
  let result;
  try {
    result = engine.check(getSource());
  } catch (e) {
    if (isTrap(e)) {
      await recoverFromTrap();
    }
    return;
  }
  renderDiagnostics(result);
}

function scheduleAutoCheck() {
  if (autoCheckTimer) clearTimeout(autoCheckTimer);
  if (!engine) return;
  autoCheckTimer = setTimeout(autoCheck, AUTOCHECK_DEBOUNCE_MS);
}

// After a Run paints output, cancel any pending auto-check and pause it so it
// doesn't immediately replace the run output. The next edit re-enables it.
function suppressAutoCheckBriefly() {
  if (autoCheckTimer) clearTimeout(autoCheckTimer);
  autoCheckSuppressed = true;
  if (suppressTimer) clearTimeout(suppressTimer);
  suppressTimer = setTimeout(() => {
    autoCheckSuppressed = false;
  }, 800);
}

// A wasm trap (from an un-unwindable panic = a Lua error: parse error, runtime
// error, error(), failed pcall) leaves the instance poisoned. Detect it,
// explain it, and transparently reload a fresh engine so the next run works.
function isTrap(e) {
  return e instanceof WebAssembly.RuntimeError || /unreachable|table index|out of bounds/i.test(String(e && e.message));
}

async function recoverFromTrap() {
  try {
    await loadEngine();
    return true;
  } catch (_) {
    return false;
  }
}

// ─────────────────────────── actions ───────────────────────────
async function doRun() {
  if (!engine) return;
  const src = getSource();
  setRunning(true);
  setStatus("running…", "running");
  await frame();
  let result;
  try {
    result = engine.run(src);
  } catch (e) {
    if (isTrap(e)) {
      // A *runtime* Lua error (error(), nil-index, failed assert/pcall) — or a
      // parse error — traps the instance, because recoverable panics can't
      // unwind on stable wasm32-unknown-unknown (panic = "abort"). We can't
      // recover the exact message text from a trap, so we say so honestly and
      // reload the engine transparently. Type-check catches most mistakes
      // *before* Run, on the path that is fully precise on this build.
      writeOutput("⚠  Runtime error.", "out-warn");
      appendOutputHtml(
        '<span class="caveat">' +
          "The in-browser build (stable <code>wasm32-unknown-unknown</code>, " +
          "<code>panic=abort</code>) can run and type-check, but it <b>cannot capture the " +
          "exact text of a runtime error</b> — that path traps the WebAssembly instance. " +
          "<br><br>" +
          "Most mistakes are caught precisely by <b>Type-check</b> " +
          "(it runs automatically as you edit). For full runtime-error messages and " +
          "stack traces, the <code>luaur</code> CLI prints them in full." +
          "<br><br>" +
          "The engine has been reloaded — the next Run works normally." +
          "</span>"
      );
      setStatus("runtime error", "error");
      suppressAutoCheckBriefly();
      await recoverFromTrap();
    } else {
      writeOutput("internal error: " + msg(e), "out-err");
      setStatus("error", "error");
    }
    setRunning(false);
    return;
  }
  // run() returns the captured print output and any error as *separate* fields,
  // so the success/failure verdict comes from `error` being non-empty — never
  // from scanning the output text. (The old content heuristic painted any run
  // whose output merely contained the word "error" — e.g. iterating `_G`, whose
  // globals include one literally named `error` — as a failure, and missed
  // compile errors whose text lacked the magic words.) A genuine runtime error
  // traps the instance and is handled in the catch above, so a non-empty `error`
  // here is a compile/load error.
  const output = result.output;
  const error = result.error;
  result.free();
  if (error) {
    let text = output;
    if (text && !text.endsWith("\n")) text += "\n";
    text += error;
    writeOutput(text, "out-err");
    setStatus("error", "error");
  } else if (output.trim() === "") {
    writeOutput("(no output — the script produced no print results)", "out-meta");
    setStatus("ran ok", "ready");
  } else {
    writeOutput(output, "out-ok");
    setStatus("ran ok", "ready");
  }
  // Keep the run output on screen; don't let a pending auto-check overwrite it.
  suppressAutoCheckBriefly();
  setRunning(false);
}

async function doCheck() {
  if (!engine) return;
  const src = getSource();
  setRunning(true);
  setStatus("checking…", "running");
  await frame();
  let result;
  try {
    result = engine.check(src);
  } catch (e) {
    if (isTrap(e)) {
      writeOutput(
        "The analyzer hit an unrecoverable error on this input (it may use a\n" +
          "code path that relies on stack unwinding, unavailable on this wasm\n" +
          "target). The engine has been reloaded.",
        "out-err"
      );
      setStatus("error", "error");
      await recoverFromTrap();
    } else {
      writeOutput("internal error: " + msg(e), "out-err");
      setStatus("error", "error");
    }
    setRunning(false);
    return;
  }
  // renderDiagnostics paints the (deduplicated, line-grouped) report and sets
  // the status pill itself.
  renderDiagnostics(result);
  setRunning(false);
}

function doClear() {
  writeOutput("", "");
  if (engine) setStatus("ready", "ready");
}

// helpers
const msg = (e) => (e && e.message ? e.message : String(e));
const frame = () => new Promise((r) => requestAnimationFrame(() => r()));
function setRunning(on) {
  elRun.disabled = on || !engine;
  elCheck.disabled = on || !engine;
}

// ─────────────────────────── boot ───────────────────────────
function populateExamples() {
  for (const [key, ex] of Object.entries(EXAMPLES)) {
    const opt = document.createElement("option");
    opt.value = key;
    opt.textContent = ex.label;
    elSelect.appendChild(opt);
  }
  elSelect.value = DEFAULT_EXAMPLE;
  elSelect.addEventListener("change", () => {
    const ex = EXAMPLES[elSelect.value];
    if (ex) {
      setSource(ex.source);
      doClear();
      scheduleAutoCheck();
    }
  });
}

async function boot() {
  populateExamples();
  editor = makeEditor(EXAMPLES[DEFAULT_EXAMPLE].source);

  elRun.addEventListener("click", doRun);
  elCheck.addEventListener("click", doCheck);
  elClear.addEventListener("click", doClear);

  try {
    // loadEngine() instantiates the wasm and wires env.js to its linear memory.
    await loadEngine();
    elRun.disabled = false;
    elCheck.disabled = false;
    setStatus("ready", "ready");
    // Type-check the initial document right away (the path that is fully precise
    // on stable wasm32-unknown-unknown) and paint the result into the output.
    autoCheck();
  } catch (e) {
    elOutput.classList.remove("is-booting");
    setStatus("wasm failed", "error");
    writeOutput(
      "Failed to load the WebAssembly engine.\n" + (e && e.message ? e.message : e),
      "out-err"
    );
    console.error(e);
  }
}

boot();
