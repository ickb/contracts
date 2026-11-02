# iCKB v1 core scripts

## Audit

These L1 Scripts have been both internally reviewed by individuals with deep experience in Nervos L1 and [externally audited by the Scalebit team](https://scalebit.xyz/reports/20240911-ICKB-Final-Audit-Report.pdf), an internationally recognized blockchain security team.

This repository also includes a later local executable review of the deployed release binaries in [`../ICKB-Audit-Report.md`](../ICKB-Audit-Report.md).

## Building the Scripts

The deployed binaries in `build/release` were built with capsule 0.10.5 and cross `6982b6c`. For each contract, [`capsule build --release`](https://github.com/nervosnetwork/capsule/blob/04fd58c5173bc8d9b0b658e06d7df907995068cc/src/recipe/rust.rs#L127-L190) runs `cross build -p <contract> --release` in the image pinned by `Cross.toml`, with two `--remap-path-prefix` flags, and copies the result. The toolchain version matters: `rust-toolchain` pins 1.74.0, and both the rustc commit and the cargo registry path end up inside the binaries.

The same build runs without capsule or cross. With rustup's 1.74.0 toolchain, its `riscv64imac-unknown-none-elf` target, and podman, run this from this directory:

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

- The image is the `focal-20230214` tag of `Cross.toml`, pinned by digest. It provides the GCC 12.2.0 that compiles the C part of ckb-std.
- `label=disable` keeps SELinux from relabelling the mounted directories, which cross would do with its `:z` mounts.
- The mounts keep host paths unchanged, so the remapped paths embedded in the binaries match the deployed ones.
- A separate target directory keeps cargo from reusing build scripts compiled on the host, which need a newer glibc than the image has.

To check the result against the deployed bytes, copy it over `build/release` and confirm Git sees no change:

```bash
cp target/rebuild/riscv64imac-unknown-none-elf/release/{ickb_logic,limit_order,owned_owner} build/release/
git status --short build/release   # empty: byte-identical to the deployed binaries
```

The test `release_binary_hashes_match_deployment_references` pins the same data hashes.
