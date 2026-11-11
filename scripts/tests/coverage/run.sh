#!/usr/bin/env bash
# Traces which instructions of the deployed binaries the default test suite executes.
# Usage: coverage/run.sh <work_dir>     (from scripts/tests; <work_dir> is created, outside the checkout)
# Needs what the rebuild in ../../README.md needs (rustup 1.74.0 with the RISC-V target, podman),
# plus llvm-objdump, llvm-objcopy and patch. Leaves in <work_dir>:
#   symbols/<name>.sym   the binaries relinked with symbols, loaded sections equal to the deployed ones
#   scripts/             a copy of this workspace built against the instrumented VM
#   default.trace        the trace of `cargo test`
# Trace more runs by setting ICKB_COV_FILE=<file> on any test command run from <work_dir>/scripts/tests.
set -euo pipefail

work=$(realpath -m "$1")
here=$(cd "$(dirname "$0")" && pwd)
scripts=$(cd "$here/../.." && pwd)
image=docker.io/nervos/ckb-riscv-gnu-toolchain@sha256:5732afe996b2b88a37ed3c0e4deb948e9d2a6936fb4b52d4e7a0dc5ad330c306
contracts="ickb_logic limit_order owned_owner"
mkdir -p "$work/build" "$work/symbols"

# 1. Symbols. The deployed build strips them at link time, so repeat it keeping the object files
#    and the linker command, then link again without --strip-all. A profile that keeps symbols
#    (strip = false, or debug info) produces different code bytes, so it cannot stand in.
podman run --rm --security-opt label=disable --userns keep-id \
  -v "$HOME/.cargo:$HOME/.cargo" -v "$HOME/.rustup:$HOME/.rustup:ro" -v "$scripts:$scripts:ro" -v "$work:$work" -w "$scripts" \
  -e HOME="$HOME" -e CARGO_HOME="$HOME/.cargo" -e CARGO_TARGET_DIR="$work/build" -e work="$work" -e contracts="$contracts" \
  -e PATH="$HOME/.rustup/toolchains/1.74.0-x86_64-unknown-linux-gnu/bin:/riscv/bin:/usr/bin:/bin" \
  -e RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$scripts=" \
  "$image" bash -euc '
    for p in $contracts; do
      cargo rustc --locked -p $p --release --target riscv64imac-unknown-none-elf -- -C save-temps --print link-args > $work/build/$p.link
      # Drop the exported-symbols object (it only exists while rustc runs) and the strip flag.
      sed -E "s# \"/tmp/rustc[A-Za-z0-9]+/symbols.o\"##; s# \"--strip-all\"##; s#(\"-o\" \")[^\"]*\"#\1$work/symbols/$p.sym\"#" $work/build/$p.link > $work/build/$p.relink.sh
      bash $work/build/$p.relink.sh
    done'
for p in $contracts; do
  for section in .text .rodata .eh_frame .data .sdata; do
    llvm-objcopy -O binary --only-section=$section "$scripts/build/release/$p" "$work/build/deployed.bin"
    llvm-objcopy -O binary --only-section=$section "$work/symbols/$p.sym" "$work/build/symbols.bin"
    cmp "$work/build/deployed.bin" "$work/build/symbols.bin" || { echo "$p $section differs from the deployed binary" >&2; exit 1; }
  done
done

# 2. The instrumented VM: the registry copy of the version Cargo.lock pins, plus the patch.
grep -A1 '^name = "ckb-vm"$' "$scripts/Cargo.lock" | grep -qx 'version = "0.24.15"' || { echo "Cargo.lock no longer pins ckb-vm 0.24.15; rebase the patch" >&2; exit 1; }
vm=$(ls -d "$HOME"/.cargo/registry/src/*/ckb-vm-0.24.15 | head -1)
rm -rf "$work/ckb-vm" && cp -r "$vm" "$work/ckb-vm"
patch -s -p1 -d "$work/ckb-vm" < "$here/ckb-vm-0.24.15.patch"

# 3. A copy of the workspace that builds against it, so the checkout and its Cargo.lock stay untouched.
rm -rf "$work/scripts" && mkdir "$work/scripts"
tar -C "$scripts" --exclude=./target -cf - . | tar -C "$work/scripts" -xf -
printf '\n[patch.crates-io]\nckb-vm = { path = "%s" }\n' "$work/ckb-vm" >> "$work/scripts/Cargo.toml"

# 4. Trace the default suite and report.
rm -f "$work/default.trace"
cd "$work/scripts/tests"
ICKB_COV_FILE="$work/default.trace" cargo test
cargo run -q --bin coverage -- "$scripts/build/release" "$work/symbols" "$work/default.trace"
