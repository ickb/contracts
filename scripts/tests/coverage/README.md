# Instruction Coverage

This directory measures which machine instructions of the deployed binaries in `../../build/release` a test run executes. The tests in [`../src/tests/fuzz/reach.rs`](../src/tests/fuzz/reach.rs) exist because of what it showed.

## Method

Stripped binaries have no function names, while a build profile that keeps the names produces different code bytes. So `run.sh` repeats the deployed build while keeping the object files and the linker command, then links them again without `--strip-all`. It checks that every loaded section of the result equals the deployed one byte for byte.

The trace comes from [`ckb-vm-0.24.15.patch`](ckb-vm-0.24.15.patch), applied to a copy of the VM the suite runs on. The asm interpreter executes only instructions it first decoded into a trace, so the patch records each decoded instruction, once per process, with the hash of the program loaded last. A decoded trace runs to its end unless the VM stops inside it, on a memory fault or when cycles run out. Such a stop would over-count by the rest of one trace, 16 instructions at most. A fused instruction pair is recorded with its full length.

The patched VM only builds in a copy of this workspace, so the checkout and `Cargo.lock` stay as they are.

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
cd -
python3 -I coverage/coverage.py ../build/release /tmp/ickb-coverage/symbols /tmp/ickb-coverage/*.trace
```

`coverage.py` takes three options:

- `--functions`: lists every function not fully executed.
- `--runs`: prints the never-executed instructions that end neither in a panic nor in an allocation failure, as disassembly.
- `--annotate <binary> <regex>`: prints the matching functions with `--` on each never-executed instruction.

## Division Branches

`division_paths.py` ports the 128-bit division that the compiler links into `iCKB Logic`. It searches for an amount and an accumulated rate per branch. `python3 -I coverage/division_paths.py check src/tests/fuzz/reach.rs` confirms that each row of `DIVISION_ROWS` takes the branch its label names.
