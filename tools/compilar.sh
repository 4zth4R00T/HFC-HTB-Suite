#!/usr/bin/env bash
set -euo pipefail

raiz="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
crate="$raiz/rust/htb-cli"
dist="${1:-$raiz/dist}"
release="${2:-$raiz/release}"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
trabajo="${HTB_BUILD_DIR:-$HOME/.cache/htb-build}"
binario="$(awk '/^\[\[bin\]\]/{b=1} b&&/^name *=/{gsub(/[" ]/,"",$0); sub(/^name=/,"",$0); print; exit}' "$crate/Cargo.toml")"
declare -A plataformas=(
  [x86_64-unknown-linux-gnu]="linux-x86_64"
  [aarch64-unknown-linux-gnu]="linux-aarch64"
  [x86_64-pc-windows-gnu]="windows-x86_64"
)
objetivos=(x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-pc-windows-gnu)

export CARGO_TARGET_DIR="$trabajo/target-$binario"
export TMPDIR="$trabajo/tmp"
mkdir -p "$TMPDIR" "$dist" "$release"

banderas=(
  "--remap-path-prefix=$HOME/="
  "--remap-path-prefix=$rustup_home/=rust/"
  "--remap-path-prefix=$cargo_home/registry/src/=deps/"
  "--remap-path-prefix=$cargo_home/git/checkouts/=deps/"
  "--remap-path-prefix=$crate/="
)
CARGO_ENCODED_RUSTFLAGS="$(IFS=$'\x1f'; printf '%s' "${banderas[*]}")"
export CARGO_ENCODED_RUSTFLAGS

cd "$crate"
registro="$trabajo/compilar-$binario.log"
: > "$registro"
for t in "${objetivos[@]}"; do
  if ! cargo build --release --locked --bins --target "$t" >> "$registro" 2>&1; then
    printf 'fallo compilando %s; detalle en %s\n' "$t" "$registro" >&2
    exit 1
  fi
done

avisos="$(grep -c '^warning' "$registro" || true)"
sumas_dist=""
sumas_release=""
informe=""
for t in "${objetivos[@]}"; do
  plat="${plataformas[$t]}"
  ext=""
  [ "$t" = x86_64-pc-windows-gnu ] && ext=".exe"
  mkdir -p "$dist/$plat"
  cp "$CARGO_TARGET_DIR/$t/release/$binario$ext" "$dist/$plat/$binario$ext"
  sumas_dist="$sumas_dist$(cd "$dist" && sha256sum "$plat/$binario$ext")"$'\n'
  if [ -n "$ext" ]; then
    paquete="$binario-$plat.zip"
    (cd "$dist/$plat" && zip -q -X -j "$release/$paquete.nuevo" "$binario$ext")
  else
    paquete="$binario-$plat.tar.gz"
    tar --owner=0 --group=0 --numeric-owner --mtime='@0' -C "$dist/$plat" -czf "$release/$paquete.nuevo" "$binario"
  fi
  mv "$release/$paquete.nuevo" "$release/$paquete"
  sumas_release="$sumas_release$(cd "$release" && sha256sum "$paquete")"$'\n'
  restos="$(strings -n 4 "$dist/$plat/$binario$ext" | grep -cF -e "$HOME" -e "/home/" || true)"
  informe="$informe$plat: $(du -h "$dist/$plat/$binario$ext" | cut -f1), rutas locales en el binario: $restos"$'\n'
done
printf '%s' "$sumas_dist" > "$dist/SHA256SUMS"
printf '%s' "$sumas_release" > "$release/SHA256SUMS"

printf '%s %s, %s avisos de compilacion\n%sSHA256SUMS en %s y %s\n' "$binario" "$(awk -F'"' '/^version/{print $2; exit}' "$crate/Cargo.toml")" "$avisos" "$informe" "$dist" "$release"
