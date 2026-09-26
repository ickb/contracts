# Test Layout

`src/tests.rs` keeps shared constants plus the top-level module wiring.

- `src/tests/fixtures.rs`: script builders, DAO/header helpers, and the small repeated setup helpers used across suites.
- `src/tests/encoders.rs`: receipt, owner-distance, witness, and limit-order data encoders.
- `src/tests/signing.rs`: sighash signing helpers for whole-transaction-binding tests.
- `src/tests/replay_helpers.rs`: live-shape parsers and batch builders used by replay-heavy cases.
- `src/tests/helpers.rs`: focused unit tests for shared encoders.
- `src/tests/ickb_logic.rs`, `owned_owner.rs`, `limit_order.rs`, `replay.rs`: thin suite roots that wire topic-focused files under `src/tests/ickb_logic/`, `owned_owner/`, `limit_order/`, and `replay/`.

Some replay tests intentionally rebuild live transaction shapes instead of minimizing the fixture. Those cases keep the audit claims tied to realistic batching, witness layout, header dependencies, and cross-script interaction patterns.

Current strong-lock coverage is direct for `secp256k1_blake160_sighash_all`, which the signed-flow tests use as the representative whole-transaction-binding lock. This repo does not include a `QRL` fixture or harness, so conclusions for other strong-lock deployments rely on the same binding property and deployment assumptions rather than a separate replay in this suite.

`ckb-testtool`'s `Context::verify_tx` executes `OutputsDataVerifier`, which checks that the output and output-data counts match, and then executes the transaction scripts through `TransactionScriptsVerifier`. It does not run the full node non-contextual or contextual transaction verifiers, so it does not establish checks such as occupied capacity, `since`, or cellbase maturity. Tests therefore verify through `VerifyRealistic::verify` in `src/tests.rs`, which first holds every input and output to its occupied capacity and then holds every accepted DAO withdrawal to the node's lock-size rule, so no fixture builds a cell or withdrawal the node would reject. Other consensus-level conclusions rely on CKB source evidence rather than `Context::verify_tx` alone.
