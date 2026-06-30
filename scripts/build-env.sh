#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEP_PREFIX="$ROOT/.deps/prefix"

export PKG_CONFIG_PATH="$DEP_PREFIX/usr/lib/x86_64-linux-gnu/pkgconfig:$DEP_PREFIX/usr/share/pkgconfig"
export PKG_CONFIG_ALLOW_SYSTEM_CFLAGS=1
export PKG_CONFIG_ALLOW_SYSTEM_LIBS=1
export CPATH="$DEP_PREFIX/usr/include${CPATH:+:$CPATH}"
export C_INCLUDE_PATH="$DEP_PREFIX/usr/include${C_INCLUDE_PATH:+:$C_INCLUDE_PATH}"
export CPLUS_INCLUDE_PATH="$DEP_PREFIX/usr/include${CPLUS_INCLUDE_PATH:+:$CPLUS_INCLUDE_PATH}"

LIB_DIR="$DEP_PREFIX/usr/lib/x86_64-linux-gnu"
SYSTEM_LIB_DIR="/usr/lib/x86_64-linux-gnu"

if [[ -d "$LIB_DIR" ]]; then
  for so in "$LIB_DIR"/*.so; do
    [[ -L "$so" ]] || continue
    target="$(readlink "$so")"
    if [[ "$target" != /* && ! -e "$LIB_DIR/$target" && -e "$SYSTEM_LIB_DIR/$target" ]]; then
      ln -sf "$SYSTEM_LIB_DIR/$target" "$LIB_DIR/$target"
    fi
  done
fi

export LIBRARY_PATH="$LIB_DIR:$DEP_PREFIX/usr/lib:$SYSTEM_LIB_DIR${LIBRARY_PATH:+:$LIBRARY_PATH}"
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-rpath,$SYSTEM_LIB_DIR"

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

if [[ -f "$HOME/.nvm/nvm.sh" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.nvm/nvm.sh"
fi

exec "$@"