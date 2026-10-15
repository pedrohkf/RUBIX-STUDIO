#!/usr/bin/env python3
"""Mede o quanto de um projeto Rojo/Luau a engine já suporta.

Uso: python3 harness/compat_report.py <pasta-do-projeto> [saida.md]

Varre os .luau/.lua, conta GetService("X"), Instance.new("X") e dependências de
assets (rbxassetid://), compara com harness/supported_api.json e escreve um
relatório em Markdown. Itens fora do JSON contam como "missing".
"""
import collections
import json
import pathlib
import re
import sys

SVC = re.compile(r'GetService\(\s*["\'](\w+)["\']\s*\)')
CLS = re.compile(r'Instance\.new\(\s*["\'](\w+)["\']')
ASSET = re.compile(r'rbxassetid://\d+|rbxasset://[\w/.\-]+')

STATUS_ORDER = ["ok", "planned", "stub", "missing"]


def scan(project: pathlib.Path):
    services, classes, assets = collections.Counter(), collections.Counter(), collections.Counter()
    files = 0
    for path in sorted(project.rglob("*")):
        if path.suffix not in (".luau", ".lua") or ".git" in path.parts:
            continue
        files += 1
        text = path.read_text(encoding="utf-8", errors="replace")
        services.update(SVC.findall(text))
        classes.update(CLS.findall(text))
        assets.update(ASSET.findall(text))
    return files, services, classes, assets


def table(title, counter, table_status):
    lines = [f"## {title}", "", "| Item | Usos | Status |", "|---|---|---|"]
    for name, count in counter.most_common():
        lines.append(f"| {name} | {count} | {table_status.get(name, 'missing')} |")
    lines.append("")
    return lines


def share(counter, table_status):
    total = sum(counter.values()) or 1
    by_status = collections.Counter()
    for name, count in counter.items():
        by_status[table_status.get(name, "missing")] += count
    return {s: round(100 * by_status[s] / total, 1) for s in STATUS_ORDER}, total


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    project = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else None
    api_path = pathlib.Path(__file__).with_name("supported_api.json")
    api = json.loads(api_path.read_text(encoding="utf-8"))

    files, services, classes, assets = scan(project)
    svc_pct, svc_total = share(services, api["services"])
    cls_pct, cls_total = share(classes, api["classes"])

    lines = [
        f"# Compatibilidade: {project.name}",
        "",
        f"- Arquivos Luau analisados: {files}",
        f"- Usos de GetService: {svc_total} | Instance.new: {cls_total}",
        f"- Dependências de assets (rbxassetid/rbxasset): {sum(assets.values())} em {len(assets)} ids distintos",
        "",
        "## Resumo (percentual dos usos por status)",
        "",
        "| Grupo | ok | planned | stub | missing |",
        "|---|---|---|---|---|",
        f"| Serviços | {svc_pct['ok']}% | {svc_pct['planned']}% | {svc_pct['stub']}% | {svc_pct['missing']}% |",
        f"| Classes | {cls_pct['ok']}% | {cls_pct['planned']}% | {cls_pct['stub']}% | {cls_pct['missing']}% |",
        "",
        "Só `ok` conta como pronto. `stub` aceita a chamada sem efeito real.",
        "",
    ]
    lines += table("Serviços", services, api["services"])
    lines += table("Classes criadas por Instance.new", classes, api["classes"])
    if assets:
        lines += table("Assets externos", assets, {})
        lines.append("Assets por ID não podem ser baixados pela engine: precisam de substituto próprio.\n")

    report = "\n".join(lines)
    if out:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(report, encoding="utf-8")
        print(f"relatório escrito em {out}")
    print(report.split("## Serviços")[0])
    return 0


if __name__ == "__main__":
    sys.exit(main())
