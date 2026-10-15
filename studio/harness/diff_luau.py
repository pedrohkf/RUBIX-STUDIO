#!/usr/bin/env python3
"""Teste diferencial de Luau: roda os mesmos scripts gerados em duas implementações.

Uso:
  python3 harness/diff_luau.py --seed 1 --count 100 \
      --ref "lune run {file}" \
      --ours "cargo run -q -p lua -- {file}"

{file} é substituído pelo caminho do script. Os scripts usam só aritmética inteira,
strings e tabelas, para que diferenças venham do runtime e não do script.
Divergências ficam em harness/out/diff/ (script, saída de cada lado) e fazem o
programa sair com código 1.
"""
import argparse
import pathlib
import random
import subprocess
import sys

OUT = pathlib.Path(__file__).resolve().parent / "out" / "diff"


def gen_script(rng: random.Random) -> str:
    lines = ["local t = {}"]
    for i in range(rng.randint(5, 15)):
        kind = rng.choice(["arith", "str", "tbl", "cmp"])
        a, b = rng.randint(-50, 50), rng.randint(1, 50)
        if kind == "arith":
            op = rng.choice(["+", "-", "*", "//", "%"])
            lines.append(f"print({a} {op} {b})")
        elif kind == "str":
            word = "".join(rng.choice("abcxyz") for _ in range(rng.randint(1, 6)))
            lines.append(f'print(string.upper("{word}"), string.rep("{word}", {b % 4}), #"{word}")')
        elif kind == "tbl":
            lines.append(f"table.insert(t, {a})")
            lines.append("table.sort(t)")
            lines.append('print(#t, table.concat(t, ","))')
        else:
            lines.append(f"print({a} < {b}, {a} == {b}, math.max({a}, {b}), math.min({a}, {b}))")
    return "\n".join(lines) + "\n"


def run(template: str, path: pathlib.Path):
    cmd = template.replace("{file}", str(path))
    try:
        proc = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
        return proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "timeout"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--count", type=int, default=100)
    parser.add_argument("--ref", required=True, help="comando do oráculo (Lune)")
    parser.add_argument("--ours", required=True, help="comando da nossa implementação")
    args = parser.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)
    rng = random.Random(args.seed)
    divergences = 0
    for i in range(args.count):
        script = OUT / f"case_{args.seed}_{i}.luau"
        script.write_text(gen_script(rng), encoding="utf-8")
        ref_code, ref_out, ref_err = run(args.ref, script)
        our_code, our_out, our_err = run(args.ours, script)
        if (ref_code, ref_out) != (our_code, our_out):
            divergences += 1
            (OUT / f"case_{args.seed}_{i}.ref.txt").write_text(ref_out + ref_err, encoding="utf-8")
            (OUT / f"case_{args.seed}_{i}.ours.txt").write_text(our_out + our_err, encoding="utf-8")
            print(f"DIVERGE: {script}")
        else:
            script.unlink()
    print(f"seed={args.seed} casos={args.count} divergências={divergences}")
    return 1 if divergences else 0


if __name__ == "__main__":
    sys.exit(main())
