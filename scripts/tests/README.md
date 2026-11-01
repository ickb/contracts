# Tests

The suite runs the deployed binaries in `../build/release`. Each module's header comment explains what it covers; this file maps them and says how to run them.

## Running

Run `cargo test --locked` from this directory. rustup reads `rust-toolchain` here, so the tests build on Rust 1.95 with the current CKB crates, while the contracts keep the 1.74 of `../rust-toolchain` that their deployed bytes need. Both share `../Cargo.lock`, which must keep lock format `version = 3`: the contracts' cargo 1.74 cannot read version 4, and newer cargo writes version 4 whenever it changes the lock, so set that line back after a dependency change.

Long runs, best in `--release`:

- More fuzz cases: `ICKB_FUZZ_CASES`, `ICKB_FUZZ_SEED`, and `ICKB_FUZZ_CASE_SEED` to replay one case from the seed a failure prints.
- The exhaustive search: `ICKB_EXHAUSTIVE_SHARD=i/n cargo test --locked --release -- --ignored exhaustive_small_scope`, one process per shard.
- The chain replay: fetch fixtures with `mainnet/ickb_mainnet.py`, then `ICKB_MAINNET_FIXTURES=<dir>/fixtures.txt cargo test --locked --release -- --ignored mainnet`.

## Layout

- `src/tests.rs`: `VerifyRealistic`, through which every test verifies. It adds the node's own transaction verifiers and the invariant oracle to every transaction the scripts accept.
- `src/tests/tx.rs`: the one way tests build transactions, as plain cell values assembled by `TxSpec`.
- `src/tests/ickb_logic/`, `owned_owner/`, `limit_order/`: hand-written scenarios per script, including every attack and boundary case the audit report cites.
- `src/tests/replay/`: transaction shapes rebuilt from observed chain flows.
- `src/tests/fuzz.rs` and `src/tests/fuzz/`: the structured fuzzer (`fuzz.rs`, `generate.rs`, `mutate.rs`), the invariant oracle (`oracle.rs`), the exhaustive search, the liveness tests and the chain replay (`mainnet.rs`).
- `src/tests/protocol_vectors.rs`: replays the golden vectors of `../vector-gen` against the release binaries.
- `src/tests/fixtures.rs`, `encoders.rs`, `signing.rs`, `replay_helpers.rs`: deployment fixtures, data encoders, secp signing and mainnet-shape helpers; `helpers.rs` unit-tests the encoders.
