# vector-gen

Golden protocol vectors for the iCKB stack's TypeScript contract oracle: the values the contracts' own Rust logic gives each case. The cases and their expected values live in `src/vectors.rs`; `src/main.rs` writes them as JSON.

`src/vectors.rs` compiles `c256.rs` and `ickb_logic/constants.rs` straight from the contract sources and holds byte-exact copies of the private `deposit_to_ickb` and `validate`; `drift_checks` fails if a copy stops matching its source. The contract tests include the same file and replay its rows against the release binaries (`../tests/src/tests/protocol_vectors.rs`), skipping by name the two that describe cells consensus cannot create, so `cargo test -p tests` checks the vectors but does not build this writer.

The crate stays outside the contracts workspace (see `Cargo.toml`). To regenerate the stack's fixture from a checkout next to this one:

```sh
cargo run --release --locked -- ../../../stack/testkit/fixtures/protocol_vectors.json
cd ../../../stack && pnpm exec prettier --write testkit/fixtures/protocol_vectors.json
```

The writer emits one object per line; the stack keeps the file Prettier-formatted, so only the second step makes the result byte-identical to the committed fixture.
