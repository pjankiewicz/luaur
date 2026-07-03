//! Full reproduction of issue #6 as reported: repeated `check_with_definitions`
//! calls and a single call with a realistic (larger) host-SDK definition file.
//! Complements `miri_issue6.rs` (which walks the minimal crashing path) with the
//! reporter's exact scenarios. Crashed with SIGSEGV on rustc 1.88 before the
//! pack-unification (4b5c7abd) and definition-module retention (eb2c1d6d) fixes.
#![cfg(feature = "typecheck")]

const REALISTIC_DEFS: &str = r#"
declare function log(message: any): ()

declare json: {
	encode: (value: any) -> string,
	decode: (s: string) -> any,
}

declare time: {
	format: (args: { unix: number, format: string, timezone: string? }) -> { ok: boolean, data: string?, error: string? },
	now: (args: { timezone: string? }?) -> { ok: boolean, data: { iso: string, unix: number, weekday: string, timezone: string, offset_minutes: number }?, error: string? },
}

declare ai: {
	complete: (args: { prompt: string, strength: string? }) -> { ok: boolean, data: string?, error: string? },
	complete_json: (args: { prompt: string, schema: any?, strength: string? }) -> { ok: boolean, data: any?, error: string? },
	filter: (args: { items: {any}, criteria: string, strength: string? }) -> { ok: boolean, data: {any}?, error: string? },
	map: (args: { items: {any}, instruction: string, strength: string? }) -> { ok: boolean, data: {any}?, error: string? },
}

declare kv: {
	set: (key: string, value: any) -> { ok: boolean, error: string? },
	get: (key: string) -> { ok: boolean, data: any? },
	delete: (key: string) -> { ok: boolean, data: boolean? },
	list: () -> { ok: boolean, data: {string}? },
	incr: (key: string, delta: number?) -> { ok: boolean, data: number?, error: string? },
}

declare composio: {
	list_integrations: () -> { ok: boolean, data: {{ slug: string, status: string }}?, error: string? },
	list_tools: (args: { integration_slug: string, search: string?, limit: number? }) -> { ok: boolean, data: {{ tool_slug: string, name: string, description: string }}?, error: string? },
	get_schema: (args: { tool_slug: string }) -> { ok: boolean, data: any?, error: string? },
	execute: (args: { integration_slug: string, tool_slug: string, params: any?, text: string? }) -> { ok: boolean, data: any?, error: string?, error_type: string? },
}

declare event: { [string]: any }
declare task: { [string]: any }
declare params: { [string]: any }
"#;

// Repro A — the SECOND call used to segfault (first succeeded).
#[test]
fn repeated_check_with_definitions() {
    let defs = "declare function log(message: any): ()";
    for _ in 0..5 {
        luaur_rt::check_with_definitions("return true", defs)
            .expect("clean script must type-check without diagnostics");
    }
}

// Repro B — a SINGLE call used to segfault when the definition file is non-trivial.
#[test]
fn single_check_with_realistic_definitions() {
    luaur_rt::check_with_definitions("return true", REALISTIC_DEFS)
        .expect("clean script must type-check without diagnostics");
}

// Both at once: repeated calls with the realistic definition file, and the
// declared globals actually used by the checked script.
#[test]
fn repeated_realistic_definitions_with_use() {
    let src = r#"
local r = kv.get("counter")
if r.ok then
    log(r.data)
end
return json.encode({ now = time.now(nil) })
"#;
    for _ in 0..3 {
        luaur_rt::check_with_definitions(src, REALISTIC_DEFS)
            .expect("clean script must type-check without diagnostics");
    }
}
