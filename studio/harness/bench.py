#!/usr/bin/env python3
"""Benchmark simples com detecção de regressão.

Uso:
  python3 harness/bench.py                 mede e compara com harness/baseline.json
  python3 harness/bench.py --set-baseline  grava a medição atual como baseline

Mede: tempo de `cargo build --release`, tamanho do binário e, se existir um binário
`bench` em target/release, executa `bench --frames 600 --json` e lê
{"avg_fps": .., "p1_low_fps": .., "parts": N}. Regressão acima de 10% sai com código 1.
"""
import json
import pathlib
import shutil
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "harness" / "out"
BASELINE = ROOT / "harness" / "baseline.json"
LIMIT = 0.10  # 10%


def measure():
    result = {}
    if not shutil.which("cargo") or not (ROOT / "Cargo.toml").exists():
        return {"skipped": "sem cargo ou sem Cargo.toml"}
    start = time.time()
    build = subprocess.run(["cargo", "build", "--release"], cwd=ROOT, capture_output=True, text=True)
    result["build_seconds"] = round(time.time() - start, 1)
    result["build_ok"] = build.returncode == 0
    bench_bin = ROOT / "target" / "release" / "bench"
    if bench_bin.exists():
        run = subprocess.run([str(bench_bin), "--frames", "600", "--json"], capture_output=True, text=True)
        try:
            result.update(json.loads(run.stdout))
        except json.JSONDecodeError:
            result["bench_error"] = (run.stderr or run.stdout)[-300:]
    return result


def regressions(now, base):
    problems = []
    for key, worse_when_higher in (("build_seconds", True), ("avg_fps", False), ("p1_low_fps", False)):
        if key in now and key in base and base[key]:
            change = (now[key] - base[key]) / base[key]
            if (worse_when_higher and change > LIMIT) or (not worse_when_higher and change < -LIMIT):
                problems.append(f"{key}: {base[key]} -> {now[key]} ({change:+.0%})")
    return problems


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    now = measure()
    (OUT / "bench-latest.json").write_text(json.dumps(now, indent=2), encoding="utf-8")
    print(json.dumps(now, indent=2))
    if "--set-baseline" in sys.argv:
        BASELINE.write_text(json.dumps(now, indent=2), encoding="utf-8")
        print("baseline gravado")
        return 0
    if not BASELINE.exists():
        print("sem baseline ainda: rode com --set-baseline quando a medição estiver estável")
        return 0
    base = json.loads(BASELINE.read_text(encoding="utf-8"))
    problems = regressions(now, base)
    if problems:
        print("REGRESSÃO:\n  " + "\n  ".join(problems))
        return 1
    print("sem regressão acima de 10%")
    return 0


if __name__ == "__main__":
    sys.exit(main())
