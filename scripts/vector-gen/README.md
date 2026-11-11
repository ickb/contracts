# vector-gen

Golden protocol vectors for the TypeScript oracle of the iCKB stack: the values the Rust logic of the contracts gives each case. The contract tests replay the same vectors against the release binaries, so the stack checks its logic against what the chain runs.

To regenerate the fixture of the stack from a checkout next to this one:

```sh
cargo run --release --locked -- ../../../stack/testkit/fixtures/protocol_vectors.json
cd ../../../stack && pnpm exec prettier --write testkit/fixtures/protocol_vectors.json
```

The second step formats the file the way the stack keeps it, so the result matches the committed fixture byte for byte.
