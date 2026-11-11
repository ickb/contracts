# Tests

The suite runs the deployed binaries in `../build/release`. Every transaction the scripts accept also goes through the transaction verifiers of the node plus an invariant oracle. Every transaction they reject goes through the node verifiers too, unless the test names the node rule its point overlaps.

What the suite holds:

- **Scenarios:** hand-written tests per script, with every attack and boundary case the [security review](../../ICKB-Audit-Report.md) cites.
- **Fuzzing:** a structured fuzzer, an exhaustive search over small transactions, plus liveness tests for the most extreme states a user can reach.
- **Chain replay:** every iCKB transaction committed on mainnet / testnet, run again against the code cells the chain used.
- **Unreached code:** tests for the machine code nothing else runs, found by tracing [instruction coverage](coverage/README.md).
- **Golden vectors:** the values the iCKB stack checks its own logic against, from [`../vector-gen`](../vector-gen/README.md).

The header comment of each module explains what it covers.

## Running

Run `cargo test --locked` from this directory.

Long runs, best in `--release`:

- More fuzz cases: set `ICKB_FUZZ_CASES` and `ICKB_FUZZ_SEED`. `ICKB_FUZZ_CASE_SEED` replays one case from the seed a failure prints.
- The exhaustive search: `ICKB_EXHAUSTIVE_SHARD=i/n cargo test --locked --release -- --ignored exhaustive_small_scope`, one process per shard.
- The chain replay: fetch fixtures with `cargo run --locked --release --manifest-path mainnet/Cargo.toml -- fetch ../build/release <dir>`, then `ICKB_MAINNET_FIXTURES=<dir>/fixtures.txt cargo test --locked --release -- --ignored mainnet`.
- Instruction coverage of the deployed binaries: `coverage/run.sh <work_dir>`, see [`coverage/README.md`](coverage/README.md).

The tests build on Rust 1.95, while the contracts keep the 1.74 their deployed bytes need.
