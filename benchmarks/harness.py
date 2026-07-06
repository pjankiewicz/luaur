#!/usr/bin/env python3
"""Cross-engine Lua/Luau microbenchmark harness.

Runs each program in benchmarks/programs/ on every engine listed in
benchmarks/engines.json (copy engines.example.json and edit the paths),
timing wall-clock externally and checking that every engine produces the
same checksum (the programs each print one number).

Usage:
    cp engines.example.json engines.json   # then edit paths
    python3 harness.py                      # all programs
    python3 harness.py fib matmul           # a subset
    ENGINES=/tmp/engines.json python3 harness.py
    RUNS=9 REF="C++ luau" python3 harness.py
"""
import subprocess, time, statistics, json, os, sys, shutil

HERE = os.path.dirname(os.path.abspath(__file__))
PROGDIR = os.path.join(HERE, "programs")
PROGS = ["fib", "nbody", "mandel", "matmul", "tablesort", "strings"]
RUNS = int(os.environ.get("RUNS", "7"))
WARMUP = 1
REF = os.environ.get("REF", "C++ luau")
ENGINES_FILE = os.environ.get("ENGINES", os.path.join(HERE, "engines.json"))

with open(ENGINES_FILE) as f:
    ENGINES = json.load(f)
if len(sys.argv) > 1:
    PROGS = sys.argv[1:]

def resolve_cmd(cmd):
    resolved = [os.path.expandvars(os.path.expanduser(part)) for part in cmd]
    binary = resolved[0]
    if os.path.isabs(binary):
        return resolved
    if os.sep in binary or (os.altsep and os.altsep in binary):
        resolved[0] = os.path.join(HERE, binary)
    return resolved

results = {}
for prog in PROGS:
    for eng in ENGINES:
        # Each engine runs the program in its own language: Lua engines run
        # programs/<prog>.lua, the Python engine runs programs/<prog>.py, etc.
        # Set "ext" in engines.json to pick the source file (defaults to "lua").
        path = os.path.join(PROGDIR, prog + "." + eng.get("ext", "lua"))
        if not os.path.exists(path):
            results[(prog, eng["label"])] = ("ERR", "no " + os.path.basename(path))
            continue
        # Skip an engine whose binary isn't installed (e.g. you only have a few
        # of the Lua engines, or just luaur + python) instead of crashing.
        cmd = resolve_cmd(eng["cmd"])
        if shutil.which(cmd[0]) is None:
            results[(prog, eng["label"])] = ("ERR", "no binary")
            continue
        full = cmd + [path]
        ts, out, err = [], None, None
        for _ in range(WARMUP):
            subprocess.run(full, capture_output=True, text=True)
        for _ in range(RUNS):
            t0 = time.perf_counter()
            r = subprocess.run(full, capture_output=True, text=True)
            t1 = time.perf_counter()
            if r.returncode != 0:
                lines = r.stderr.strip().splitlines() or ["rc=%d" % r.returncode]
                err = lines[-1][:60]
                break
            ts.append(t1 - t0); out = r.stdout.strip()
        results[(prog, eng["label"])] = ("ERR", err) if err else (min(ts), statistics.median(ts), out)

labels = [e["label"] for e in ENGINES]
print("\n# Runtime — median wall-clock ms over %d runs (xN = vs %s; lower is faster)\n" % (RUNS, REF))
hdr = "%-11s" % "prog" + "".join("%21s" % l for l in labels)
print(hdr); print("-" * len(hdr))
for prog in PROGS:
    r = results.get((prog, REF)); refmed = r[1] if r and r[0] != "ERR" else None
    cells = []
    for l in labels:
        v = results.get((prog, l))
        if not v: cells.append("%21s" % "-")
        elif v[0] == "ERR": cells.append("%21s" % ("ERR " + v[1][:16]))
        else:
            rel = (v[1] / refmed) if refmed else None
            cells.append("%21s" % ("%.0f" % (v[1] * 1000) + (" (%.1fx)" % rel if rel else "")))
    print("%-11s" % prog + "".join(cells))

import math
BASE = os.environ.get("BASE", "luaur")
basevals = {p: results[(p, BASE)][1] for p in PROGS
            if results.get((p, BASE)) and results[(p, BASE)][0] != "ERR"}
if basevals:
    print("\n# Average across all benchmarks — geometric mean of per-benchmark time ratios (baseline = %s)\n" % BASE)
    print("%-12s %14s   %s" % ("engine", "geomean time", "meaning"))
    print("-" * 60)
    for l in labels:
        ratios = [results[(p, l)][1] / basevals[p] for p in PROGS
                  if results.get((p, l)) and results[(p, l)][0] != "ERR" and p in basevals]
        if not ratios:
            continue
        g = math.exp(sum(math.log(r) for r in ratios) / len(ratios))
        if abs(g - 1.0) < 1e-9:
            meaning = "(baseline)"
        elif g < 1:
            meaning = "%.2fx faster than %s" % (1 / g, BASE)
        else:
            meaning = "%.2fx slower than %s" % (g, BASE)
        print("%-12s %12.2fx   %s" % (l, g, meaning))

print("\n# Correctness — output checksum agreement vs %s\n" % REF)
for prog in PROGS:
    ro = results.get((prog, REF)); refval = ro[2] if ro and ro[0] != "ERR" else None
    bad = [f"{l}={results[(prog,l)][2]}" for l in labels
           if results.get((prog, l)) and results[(prog, l)][0] != "ERR" and refval is not None and results[(prog, l)][2] != refval]
    print("%-11s ref=%-14s %s" % (prog, refval, "OK" if not bad else "MISMATCH: " + "; ".join(bad)))
