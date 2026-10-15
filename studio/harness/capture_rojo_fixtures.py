#!/usr/bin/env python3
"""Captura fixtures reais do `rojo serve`: leitura inicial, patches do WebSocket e leitura final.

Uso (com `rojo serve` rodando numa CÓPIA do projeto, nunca no original):
  python3 harness/capture_rojo_fixtures.py <pasta-do-projeto-copia> [porta]

Edita temporariamente um arquivo da cópia (e cria/apaga outro) para provocar três patches
(atualização, adição, remoção), grava cada frame binário e restaura a cópia.
Precisa de `websockets` (pip install websockets) e `msgpack` (pip install msgpack).
Saída em harness/fixtures/rojo/: info.msgpack, read_before.msgpack, patch_1_update.msgpack,
patch_2_add.msgpack, patch_3_remove.msgpack, read_after.msgpack, cursors.json
"""
import json
import pathlib
import sys
import time
import urllib.request

import msgpack
from websockets.sync.client import connect

OUT = pathlib.Path(__file__).resolve().parent / "fixtures" / "rojo"


def main():
    project = pathlib.Path(sys.argv[1])
    port = sys.argv[2] if len(sys.argv) > 2 else "34872"
    base = f"http://127.0.0.1:{port}"
    OUT.mkdir(parents=True, exist_ok=True)

    info_raw = urllib.request.urlopen(f"{base}/api/rojo").read()
    (OUT / "info.msgpack").write_bytes(info_raw)
    info = msgpack.unpackb(info_raw, raw=False)
    root = info["rootInstanceId"]

    read_raw = urllib.request.urlopen(f"{base}/api/read/{root}").read()
    (OUT / "read_before.msgpack").write_bytes(read_raw)
    cursor = msgpack.unpackb(read_raw, raw=False)["messageCursor"]

    edited = project / "src" / "shared" / "HealthColor.luau"
    created = project / "src" / "shared" / "FixtureNew.luau"
    original = edited.read_bytes()
    cursors = {"start": cursor}

    def next_frame(ws, name):
        frame = ws.recv(timeout=15)
        data = frame if isinstance(frame, bytes) else frame.encode()
        (OUT / f"{name}.msgpack").write_bytes(data)
        packet = msgpack.unpackb(data, raw=False)
        cursors[name] = packet["body"]["messageCursor"]
        msgs = packet["body"]["messages"]
        print(name, "mensagens:", len(msgs),
              {k: len(v) for k, v in msgs[0].items()} if msgs else {})

    try:
        with connect(f"ws://127.0.0.1:{port}/api/socket/{cursor}") as ws:
            time.sleep(0.5)
            edited.write_bytes(original + b"\n-- editado pelo fixture\n")
            next_frame(ws, "patch_1_update")
            created.write_text('return { fixture = true }\n')
            next_frame(ws, "patch_2_add")
            created.unlink()
            next_frame(ws, "patch_3_remove")
    finally:
        edited.write_bytes(original)
        if created.exists():
            created.unlink()

    time.sleep(1.0)
    read_after = urllib.request.urlopen(f"{base}/api/read/{root}").read()
    (OUT / "read_after.msgpack").write_bytes(read_after)
    (OUT / "cursors.json").write_text(json.dumps(cursors, indent=1))
    print("fixtures gravadas em", OUT)


if __name__ == "__main__":
    main()
