# Test Layout

`src/tests.rs` keeps shared constants plus the top-level module wiring.

- `src/tests/fixtures.rs`: script builders, DAO/header helpers, and the small repeated setup helpers used across suites.
- `src/tests/encoders.rs`: receipt, owner-distance, witness, and limit-order data encoders.
- `src/tests/signing.rs`: sighash signing helpers for whole-transaction-binding tests.
- `src/tests/replay_helpers.rs`: live-shape parsers and batch builders used by replay-heavy cases.
- `src/tests/helpers.rs`: focused unit tests for shared encoders.
- `src/tests/protocol_vectors.rs`: replays the golden vectors of `../vector-gen` (the values the contracts' own Rust logic gives each case) against the release binaries, except two named vectors that describe cells consensus cannot create.
- `src/tests/fuzz.rs` with `fuzz/generate.rs`, `fuzz/mutate.rs` and `fuzz/oracle.rs`: structured fuzzing of the deployed binaries. It composes valid transaction segments, mutates them, verifies each one and checks the protocol invariants on every accepted one. The default suite runs 100 fixed-seed cases; `ICKB_FUZZ_CASES`, `ICKB_FUZZ_SEED` and `ICKB_FUZZ_CASE_SEED` drive longer local runs, best in `--release`.
- `src/tests/fuzz/exhaustive.rs`: every transaction of up to three inputs and three outputs (five cells in all) over a fixed set of boundary archetypes, checked by the same oracle. The full search is ignored and runs sharded with `ICKB_EXHAUSTIVE_SHARD=i/n`; a small slice runs in the default suite.
- `src/tests/fuzz/liveness.rs`: the largest and most extreme states a user can reach (a full withdrawal batch and its claim, long header dep lists, receipts and deposits at the soft-cap and rate edges, fulfilled, dust and below-minimum orders), each followed by the owner's next transaction, which must verify within one mainnet block and keep the invariants.
- `src/tests/fuzz/mainnet.rs`: an ignored test that replays committed chain transactions through the same oracle, from fixtures that `mainnet/ickb_mainnet.py` fetches (mainnet by default, testnet with its RPC URL). The script also checks live iCKB supply against pool deposits and receipts.
- `src/tests/ickb_logic.rs`, `owned_owner.rs`, `limit_order.rs`, `replay.rs`: thin suite roots that wire topic-focused files under `src/tests/ickb_logic/`, `owned_owner/`, `limit_order/`, and `replay/`.

Some replay tests intentionally rebuild live transaction shapes instead of minimizing the fixture. Those cases keep the audit claims tied to realistic batching, witness layout, header dependencies, and cross-script interaction patterns.

Current strong-lock coverage is direct for `secp256k1_blake160_sighash_all`, which the signed-flow tests use as the representative whole-transaction-binding lock. This repo does not include a `QRL` fixture or harness, so conclusions for other strong-lock deployments rely on the same binding property and deployment assumptions rather than a separate replay in this suite.

`ckb-testtool`'s `Context::verify_tx` executes `OutputsDataVerifier`, which checks that the output and output-data counts match, and then executes the transaction scripts through `TransactionScriptsVerifier`. It does not run the node's other transaction verifiers. Tests therefore verify through `VerifyRealistic::verify` in `src/tests.rs`, which holds every transaction the scripts accept to the node's own verifiers from `ckb-verification` 1.2.1 (the code of node v0.210.0) under mainnet's consensus built from its bundled chain spec: the non-contextual checks (version, size, empty inputs or outputs, duplicate deps, output data count, lock hash types), `CapacityVerifier`, the fee calculation with NervosDAO maximum withdraw, and `DaoScriptSizeVerifier` (lock size and output data). It also rejects an input cell spent twice, an input below its occupied capacity, and a malformed `since`. Only checks that waiting satisfies (`since` and cellbase maturity) are left out. The fuzzer and the exhaustive search discard what this check rejects before running the scripts.

The tests build on Rust 1.95 with the current CKB crates: rustup reads `rust-toolchain` here, while the contracts keep the 1.74 of `../rust-toolchain` that their deployed bytes need. Both stay in one workspace and share `../Cargo.lock`, which must keep lock format `version = 3`: the contracts' cargo 1.74 cannot read version 4, and newer cargo writes version 4 whenever it changes the lock, so set that line back after a dependency change. Run the tests from this directory with `cargo test --locked`.
