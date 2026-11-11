# Instruction Coverage

This directory shows which machine instructions of the deployed binaries in `../../build/release` a test run executes. The tests in [`../src/tests/fuzz/reach.rs`](../src/tests/fuzz/reach.rs) exist because of what it showed.

## How It Works

- **Names over the deployed bytes:** stripped binaries have no function names, while a build that keeps them changes the code. So `run.sh` repeats the deployed build, then links the same object files again without stripping. It checks that every loaded section equals the deployed one byte for byte.
- **A trace of every instruction:** [`ckb-vm-0.24.15.patch`](ckb-vm-0.24.15.patch) makes a copy of the VM the suite runs on record each instruction it decodes. That VM runs only instructions it first decoded into a trace, which then runs to its end. A run that stops mid-trace, on a memory fault or out of cycles, over-counts by 16 instructions at most.
- **No change to the checkout:** the patched VM builds in a copy of this workspace, so `Cargo.lock` stays as it is.

## Running

From `scripts/tests`, with what the rebuild in [`../../README.md`](../../README.md) needs, plus `llvm-objdump`, `llvm-objcopy` and `patch`:

```bash
coverage/run.sh /tmp/ickb-coverage
```

It prints the report for the default suite. To trace any other run, set `ICKB_COV_FILE` on a test command in the copy, then pass every trace to the report:

```bash
cd /tmp/ickb-coverage/scripts/tests
ICKB_COV_FILE=/tmp/ickb-coverage/fuzz.trace ICKB_FUZZ_CASES=150000 ICKB_FUZZ_SEED=$((0x5eed0101)) \
  cargo test --release fuzz_accepted_transactions
ICKB_COV_FILE=/tmp/ickb-coverage/chain.trace ICKB_MAINNET_FIXTURES=<dir>/fixtures.txt \
  cargo test --release -- --ignored mainnet
cargo run -q --bin coverage -- ../build/release /tmp/ickb-coverage/symbols /tmp/ickb-coverage/*.trace
cd -
```

[`../src/bin/coverage.rs`](../src/bin/coverage.rs) lists the options of the report.
