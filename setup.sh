#!/usr/bin/env bash
# Prepara o ambiente: clona as referências de código aberto e instala Lune e Rojo.
# Uso: ./setup.sh        (leva alguns minutos: compila Lune e Rojo)
set -u
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo não encontrado. No Arch: sudo pacman -S --needed rustup && rustup default stable"
  exit 1
fi
if ! command -v git >/dev/null 2>&1; then
  echo "git não encontrado: sudo pacman -S --needed git"
  exit 1
fi

mkdir -p refs reference

clone() {
  if [ -d "refs/$1/.git" ]; then
    echo "refs/$1 já existe"
  else
    git clone --depth 1 "$2" "refs/$1"
  fi
}
echo "== Referências (somente leitura)"
clone rojo     https://github.com/rojo-rbx/rojo
clone rbx-dom  https://github.com/rojo-rbx/rbx-dom
clone lune     https://github.com/lune-org/lune

echo "== Lune (oráculo de testes)"
command -v lune >/dev/null 2>&1 && echo "lune já instalado" || cargo install lune --locked

echo "== Rojo (servidor)"
command -v rojo >/dev/null 2>&1 && echo "rojo já instalado" || cargo install rojo --locked

echo "== Python (harness): msgpack e websockets"
python3 -m pip install --user --quiet msgpack websockets 2>/dev/null \
  || echo "Aviso: pip não instalou msgpack/websockets (só o capture_rojo_fixtures.py precisa). No Arch: sudo pacman -S python-msgpack python-websockets"

echo
echo "Pronto. Confira que ~/.cargo/bin está no PATH (lune --version, rojo --version) e rode:"
echo "  cd studio && cargo test --workspace      # esperado: 7 testes passando"
