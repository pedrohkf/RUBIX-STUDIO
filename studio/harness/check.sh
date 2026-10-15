#!/usr/bin/env bash
# Verificação de qualidade: formatação, clippy e testes. Saída resumida e logs em harness/out/.
set -u
cd "$(dirname "$0")/.."
OUT=harness/out
mkdir -p "$OUT"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo não encontrado: instale o Rust (rustup) antes de rodar o harness."
  exit 2
fi
if [ ! -f Cargo.toml ]; then
  echo "Sem Cargo.toml ainda: nada para verificar."
  exit 0
fi

status=0
run() {
  name=$1
  shift
  if "$@" >"$OUT/$name.log" 2>&1; then
    echo "$name: ok"
  else
    echo "$name: FALHOU (ver $OUT/$name.log)"
    status=1
  fi
}

run fmt cargo fmt --all -- --check
run clippy cargo clippy --all-targets -- -D warnings
run test cargo test --all
exit $status
