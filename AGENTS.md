# Notes for Agents

- **Frozen:** the scripts are live on mainnet and can't change. Keep `scripts/contracts`, `scripts/build`, `scripts/deployment`, `scripts/Cargo.toml`, `scripts/rust-toolchain`, `scripts/Cross.toml`, `scripts/capsule.toml` byte-identical, plus the version and checksum of every `scripts/Cargo.lock` package the contracts resolve.
- **Lock format:** contracts (Rust 1.74) and tests (Rust 1.95) share `scripts/Cargo.lock`, which must stay `version = 3`. Newer cargo rewrites it to 4 on any dependency change: set it back.
- **Dependencies:** a tool whose crates would change `scripts/Cargo.lock` gets its own crate and lock (an empty `[workspace]` table), like `scripts/tests/mainnet`. Tooling stays Rust.
- **Vulnerabilities:** a suspected one goes through [`SECURITY.md`](SECURITY.md), never into a commit, issue or pull request.
- **Writing:** docs, commits and pull requests describe the work, never the tools or models behind it.
