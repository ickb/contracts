# iCKB v1 core scripts

This directory holds the Rust source of the three scripts (`contracts/`), the binaries deployed on chain (`build/release`), their deployment records (`deployment/`) and the [tests](tests/README.md).

## Building the Scripts

The deployed binaries rebuild byte for byte from this source: Rust 1.74.0 in the RISC-V image that `Cross.toml` pins. With rustup and podman, run this from this directory:

```bash
rustup target add riscv64imac-unknown-none-elf --toolchain 1.74.0
podman run --rm --security-opt label=disable --userns keep-id \
  -v "$HOME/.cargo:$HOME/.cargo" -v "$HOME/.rustup:$HOME/.rustup:ro" -v "$PWD:$PWD" -w "$PWD" \
  -e HOME="$HOME" -e CARGO_HOME="$HOME/.cargo" -e CARGO_TARGET_DIR="$PWD/target/rebuild" \
  -e PATH="$HOME/.rustup/toolchains/1.74.0-x86_64-unknown-linux-gnu/bin:/riscv/bin:/usr/bin:/bin" \
  -e RUSTFLAGS="--remap-path-prefix=$HOME=~ --remap-path-prefix=$PWD=" \
  docker.io/nervos/ckb-riscv-gnu-toolchain@sha256:5732afe996b2b88a37ed3c0e4deb948e9d2a6936fb4b52d4e7a0dc5ad330c306 \
  bash -c 'for p in ickb_logic limit_order owned_owner; do cargo build --locked -p $p --release --target riscv64imac-unknown-none-elf; done'
```

[capsule 0.10.5](https://github.com/nervosnetwork/capsule/blob/04fd58c5173bc8d9b0b658e06d7df907995068cc/src/recipe/rust.rs#L127-L190) built the deployed binaries by running `cross build` (cross `6982b6c`) per contract in the same image, with two `--remap-path-prefix` flags. This command runs the same `cargo build` without capsule or cross:

- The image is the `focal-20230214` tag of `Cross.toml`, pinned by digest. It provides the GCC 12.2.0 that compiles the C part of ckb-std.
- `label=disable` keeps SELinux from relabelling the mounted directories, which cross would do with its `:z` mounts.
- The mounts keep host paths unchanged, so the remapped paths embedded in the binaries match the deployed ones.
- A separate target directory keeps cargo from reusing build scripts compiled on the host, which need a newer glibc than the image has.

Then check the result against the deployed bytes: Git sees no change.

```bash
cp target/rebuild/riscv64imac-unknown-none-elf/release/{ickb_logic,limit_order,owned_owner} build/release/
git status --short build/release   # empty: byte-identical to the deployed binaries
```

The test `release_binary_hashes_match_deployment_references` pins the same data hashes.
