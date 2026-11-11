# iCKB Contracts Security Review

- **Review period:** 2026-05-01 to 2026-11-10. The review keeps evolving. The repository history records each revision.
- **Reviewed contracts commit:** [`454cfa9`](https://github.com/ickb/contracts/tree/454cfa966052a621c4e8b67001718c29ee8191a2). This is the last commit that changed `scripts/contracts/**` or `scripts/Cargo.toml`.
- **Executable test evidence:** the current `scripts/tests/**` suite in this repository state.
- **Scope:** `iCKB Logic`, `Owned Owner`, `Limit Order` and the shared `utils` crate.
- **Cross-references:** the [iCKB whitepaper](https://github.com/ickb/whitepaper/blob/master/README.md) and the Nervos L1 reference implementations.
- **Prior external audit:** [Scalebit (2024-09-10)](https://scalebit.xyz/reports/20240911-ICKB-Final-Audit-Report.pdf). It reported three issues: two informational and one minor.

## Executive Summary

The review asks one question about the deployed scripts: does any transaction they accept break a protocol invariant? Four invariants carry the protocol. Each has its own section below:

1. [No iCKB from nothing](#invariant-1-no-ickb-from-nothing): iCKB supply moves only with the value of pool deposits and receipts.
2. [Receipts match deposits](#invariant-2-receipts-match-deposits): every receipt counts deposits created with it.
3. [Limit orders keep their value](#invariant-3-limit-orders-keep-their-value): a fill never lowers the value of an order at its own ratio.
4. [Withdrawal claims stay one to one](#invariant-4-withdrawal-claims-stay-one-to-one): each withdrawal request pairs with exactly one owner cell.

Each holds whenever its owning script runs. The one live issue is the known `Limit Order` confusion attack, LO-01. The scripts never see that state, because it is created without running them. The other limits are of the same kind or sit outside the contracts:

- Receiptless deposits enter the pool without running `iCKB Logic`. The paths they open cost their user more than they return. No test shows theft, duplicated principal or a profit beyond assets the caller already controls.
- The user lock binds recipients. The iCKB scripts don't. So integrations with locks that don't sign the whole transaction can have outputs redirected.
- One transaction can outgrow the memory of a script: 382 limit orders at once in the tightest case. The same cells then move in smaller transactions.

The evidence binds to the code on chain. A byte-exact rebuild gives back the deployed binaries from this source. Every accepted transaction in the suite goes through the same transaction verifiers the node runs, plus an invariant oracle. That covers hand-written scenarios, a structured fuzzer and an exhaustive small-scope search. Every committed mainnet and testnet transaction goes through the same checks, then runs again script by script. None breaks an invariant. Mutation testing measures how much a clean run proves. An instruction-level trace of the deployed binaries shows which machine code those runs execute, with tests for every remaining part that an input can reach. The compiled-in dependencies and the known defects of the compiler were checked against the same bytes. Liveness tests show that the most extreme reachable states stay spendable within one block. They also pin the size at which one transaction outgrows a script.

## Findings Summary

One known live issue remains in scope.

| ID    | Status | Component     | Finding                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| ----- | ------ | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LO-01 | Known  | `Limit Order` | `limit_order` runs only when a transaction creates or spends a master cell or spends an order. A transaction that only creates order cells runs no check, because CKB [doesn't execute output locks](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/types.rs#L642-L666). So anyone can create phantom orders pointing at no master, or fake orders pointing at the live master of someone else. A fake order can trick the owner into melting it with their master, which strands their real order. |

## Protocol Model and Assumptions

iCKB is an inflation-protected CKB token (`xUDT`) that turns NervosDAO deposits into a liquid, fungible asset. The protocol owns all CKB deposits in a shared pool. Users deposit CKB and receive iCKB. Later they can burn iCKB to withdraw from any mature deposit in that pool.

Three on-chain scripts govern that flow:

1. **iCKB Logic (dual-role):** lock script on deposit cells, type script on receipt cells. It enforces the balance equation and the deposit-receipt accounting.
2. **Owned Owner:** pairs withdrawal request cells with owner cells, so users can claim NervosDAO phase 2 withdrawals.
3. **Limit Order:** lets users exchange CKB and UDTs through order-book-style orders, without handling NervosDAO and iCKB protocol constraints themselves.

### Key Flows

**Deposit (two phases):**

- Phase 1: CKB goes into NervosDAO deposit cells (lock = iCKB Logic, type = DAO). Receipts (type = iCKB Logic) track the deposits.
- Phase 2: receipts become iCKB `xUDT` tokens at the accumulated rate (AR) of the deposit block.

**Withdrawal:**

- Phase 1: burning iCKB releases deposits from the pool into NervosDAO withdrawal requests.
- Phase 2: standard NervosDAO withdrawal (outside iCKB scope).

**Mixed transactions:** a single transaction can combine deposit phase 1, phase 2 and withdrawal operations.

The rules below explain why the invariants hold where they do. They also explain why the invariants can't reach further.

### Script Grouping

Each iCKB script has two roles: a lock on one kind of cell and a type on another. Whenever it runs in either role, it validates every cell of the transaction that uses it, not only its own group. The only states it can't check are those created by transactions where it doesn't run, because CKB doesn't execute output locks. LO-01 is the live instance. Receiptless `iCKB Logic` deposits and lock-only Owned Owner look-alikes are the others.

A transaction that spends an `iCKB Logic`-locked input and also carries a receipt input or output executes `iCKB Logic` twice, once per role (a phase 1 plus phase 2 mix without deposit inputs runs it once). CKB groups scripts by both hash and role. From [`script/src/types.rs:175-176`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/types.rs#L175-L176):

> A cell can have a lock script and an optional type script. Even they reference the same script, lock script and type script will not be grouped together.

Lock groups and type groups live in separate `BTreeMap`s ([`types.rs:583-585`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/types.rs#L583-L585)). Lock groups come from input locks only, while type groups come from both input and output types ([`types.rs:642-666`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/types.rs#L642-L666)). `iCKB Logic` uses the same `{code_hash, Data1, empty_args}` for both roles, so a mixed transaction produces one lock group for deposit cells and one type group for receipt cells. Both see the same global cell set (`Source::Input` and `Source::Output`, not group-local sources) and apply the same checks. CKB requires [every script group to pass](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/verify.rs#L180-L197), so one run can't pass while the other fails (scenario 5B).

### xUDT Owner Mode

In the deployed configuration only two `xUDT` owner-mode routes are live. Both also execute `iCKB Logic`, so owner mode stays within iCKB accounting.

iCKB `xUDT` args are built as [`[ickb_logic_hash, XUDT_ARGS_FLAGS]`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L116-L126) with [`XUDT_ARGS_FLAGS = 0x80000000`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/constants.rs#L1-L2).

The [owner-mode update](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0052-extensible-udt/0052-extensible-udt.md#owner-mode-update) of RFC 0052 has four triggers: matching `input lock`, matching `input type`, matching `output type` and a witness `owner_script` whose hash matches the owner hash in args.

In the deployed configuration, the upstream [`xudt_rce.c` flag parsing](https://github.com/nervosnetwork/ckb-production-scripts/blob/26b0b4f15bb6eeb268b70d7ae006e244b7c06649/c/xudt_rce.c#L383-L458) enables `input lock` by default. It enables `input type` when `flags & 0x80000000` is non-zero, while it enables `output type` only when `flags & 0x40000000` is non-zero. It also [falls back to the witness `owner_script` path](https://github.com/nervosnetwork/ckb-production-scripts/blob/26b0b4f15bb6eeb268b70d7ae006e244b7c06649/c/xudt_rce.c#L645-L649).

The witness `owner_script` path needs an exported [`validate` symbol](https://github.com/nervosnetwork/ckb-production-scripts/blob/26b0b4f15bb6eeb268b70d7ae006e244b7c06649/c/xudt_rce.c#L36-L36) loaded through [`ckb_dlsym`](https://github.com/nervosnetwork/ckb-production-scripts/blob/26b0b4f15bb6eeb268b70d7ae006e244b7c06649/c/xudt_rce.c#L91-L113). [`iCKB Logic`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/main.rs#L23-L35) is instead built as a CKB entrypoint (`program_entry` through `ckb_std::entry!`), not as an xUDT `validate` extension. Under the deployed iCKB args `[ickb_logic_hash, 0x80000000]`, the live owner-mode paths are:

- A receipt (input type = iCKB Logic): needed for phase 2 minting.
- A deposit (input lock = iCKB Logic): fires during withdrawal.

In both cases `iCKB Logic` runs too, as a type script for receipts or as a lock script for deposits. So `xUDT` can't enter owner mode without `iCKB Logic` also running.

### NervosDAO Interaction

The [deposit phase 1 section of the iCKB whitepaper](https://github.com/ickb/whitepaper/blob/master/README.md#deposit-phase-1) pins `NervosDAO` to a [historical `dao.c`](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c). That version enforces the known [64-output-cell limit](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L565-L591) with an [`output_withdrawing_mask` bitset](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L445). iCKB inherits the limit as a platform constraint, but its own correctness doesn't depend on it.

Key NervosDAO constraints, checked against the [`dao.c` the whitepaper pins](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c):

- A withdrawal request must sit at the [same output index](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L518-L523) as the consumed deposit.
- A withdrawal request must have [the same capacity as the deposit](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L356-L365).
- [`validate_withdrawing_cell`](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L339-L385) doesn't check the lock of a withdrawal request. Current CKB nodes still apply the [DaoScriptSizeVerifier](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/verification/src/transaction_verifier.rs#L865-L982): for deposits committed at or after block 10,000,000, the withdrawing lock must have exactly the serialized size of the consumed deposit lock. Older deposits are exempt.
- AR is read from [`deposit_data.dao[8]`](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L282-L283), matching the iCKB [`AR_OFFSET = 160 + 8`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L51).

### Header Access

Header access explains why iCKB uses a two-phase deposit flow. From [`script/src/syscalls/load_header.rs`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/syscalls/load_header.rs):

- For `Source::Input`: it returns the header of the block holding the transaction that created the cell. That block hash must appear in [`header_deps`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/syscalls/load_header.rs#L61-L66).
- For `Source::Output`: it always returns `INDEX_OUT_OF_BOUND` ([`load_header.rs:79`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/syscalls/load_header.rs#L79)). So deposits need two phases.

### Authorization Boundary

Several candidate issues turn on the boundary between lock-script authorization and type-script accounting. With the [lock script](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#lock-script) / [type script](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#type-script) split, the user lock enforces ownership while `ickb_logic` enforces value conservation. Its [balance equation](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L28-L34) and [cell classification](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L69-L81) never look at who receives the output `iCKB` or the phase-1 DAO claim.

Under the [current assumption that user locks bind the whole transaction](https://github.com/ickb/whitepaper/blob/master/README.md#ancillary-scripts), that split is acceptable: the user signature commits to the full transaction. The deployment model of this review assumes current user-facing iCKB flows use strong transaction-binding locks. It doesn't treat delegated or adopted `OTX` flows as in-scope present-day integrations.

This stays an integration assumption, not a full inventory of connector locks. The stack interface being prepared filters connectors by `SignerType.CKB`. It doesn't independently prove how each signer binds outputs. The public SDK also accepts signers or scripts from the caller. Any integration using a weaker lock inherits the reassignment behavior shown below.

The weak-lock tests in this report don't replay a current wallet path. They model future or custom integrations (delegated, `OTX` or other integrations whose locks don't bind outputs), where recipient binding is an integration invariant, not a guarantee of the iCKB contracts.

Executed regressions show why that stays a boundary case, not a current finding:

- **Weak-lock behavior:** recipient reassignment is possible in both `iCKB Logic` and `Owned Owner`. The [phase-2 weak-lock redirect test](scripts/tests/src/tests/ickb_logic/phase2_recipient_binding.rs) and the [withdrawal weak-lock redirect test](scripts/tests/src/tests/owned_owner/weak_lock_output_rebinding.rs) show that path.
- **Signed phase-2 minting:** once `sighash` binds the full transaction, phase-2 redirects stop working. The [phase-2 sighash binding test](scripts/tests/src/tests/ickb_logic/phase2_recipient_binding.rs) and the [mixed phase-2 sighash binding test](scripts/tests/src/tests/ickb_logic/phase2_recipient_binding.rs) show that binding.
- **Signed withdrawals:** the same redirect fails for withdrawal outputs once `sighash` binds the transaction. The [withdrawal sighash binding test](scripts/tests/src/tests/owned_owner/weak_lock_output_rebinding.rs) and the [mixed withdrawal sighash binding test](scripts/tests/src/tests/owned_owner/weak_lock_output_rebinding.rs) show the same result.
- **Witness binding:** the [input-group, noncontiguous input-group and full-witness signing tests](scripts/tests/src/tests/signing.rs) confirm the witness-binding model of the signed path.

The same [ancillary scripts section](https://github.com/ickb/whitepaper/blob/master/README.md#ancillary-scripts) already warns that delegated and `OTX` ownership patterns have their own pitfalls. These tests use `secp256k1_blake160_sighash_all` as the representative lock that binds the whole transaction. The repository has no `QRL` fixture. So conclusions for other strong-lock deployments rest on the same binding property, not on a separate replay.

## How the Deployed Bytes Were Tested

The deployment can't be patched (see [Non-Upgradable Deployment](#non-upgradable-deployment)). So every result here is evidence about the bytes on chain, not regression protection. No check below found the deployed scripts accepting a transaction that breaks an invariant.

### Reference Repositories

| Repository                                                                                                                                    | Commit                                                                                                             |
| --------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| [ickb/contracts](https://github.com/ickb/contracts/tree/454cfa966052a621c4e8b67001718c29ee8191a2)                                             | [`454cfa9`](https://github.com/ickb/contracts/tree/454cfa966052a621c4e8b67001718c29ee8191a2)                       |
| [ickb/whitepaper](https://github.com/ickb/whitepaper/tree/master)                                                                             | [`master`](https://github.com/ickb/whitepaper/tree/master)                                                         |
| [nervosnetwork/ckb](https://github.com/nervosnetwork/ckb/tree/2592ddf0502cd4adfe886db893cccc866db3c60f)                                       | [`2592ddf`](https://github.com/nervosnetwork/ckb/tree/2592ddf0502cd4adfe886db893cccc866db3c60f)                    |
| [nervosnetwork/ckb-system-scripts](https://github.com/nervosnetwork/ckb-system-scripts/tree/814eb82c44f560dbdad2be97eb85464062920237)         | [`814eb82`](https://github.com/nervosnetwork/ckb-system-scripts/tree/814eb82c44f560dbdad2be97eb85464062920237)     |
| [nervosnetwork/ckb-production-scripts](https://github.com/nervosnetwork/ckb-production-scripts/tree/26b0b4f15bb6eeb268b70d7ae006e244b7c06649) | [`26b0b4f`](https://github.com/nervosnetwork/ckb-production-scripts/tree/26b0b4f15bb6eeb268b70d7ae006e244b7c06649) |
| [nervosnetwork/rfcs](https://github.com/nervosnetwork/rfcs/tree/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe)                                     | [`4b502ff`](https://github.com/nervosnetwork/rfcs/tree/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe)                   |

The `nervosnetwork/ckb` pin is release `v0.210.0`. It takes its genesis NervosDAO binary prebuilt from the `ckb-system-scripts` 0.5.4 crate. The `v0.5.4` tag of that repository holds a `dao.c` byte-identical to the pinned one. The last code change to `dao.c` came in an [earlier commit](https://github.com/nervosnetwork/ckb-system-scripts/blob/f25c5ae8824c4907ad94326a0113a03defab9bfc/c/dao.c). Later commits change only comments.

### Source Matches the Deployed Bytes

Building the contracts at this repository state, in the image `scripts/Cross.toml` pins, gives back all three deployed binaries byte for byte: same data hashes as `scripts/build/release`. The [build recipe](scripts/README.md#building-the-scripts) runs the same `cargo build` that capsule 0.10.5 ran, without capsule or cross. So the source analysis in this report and the mutation testing below bind to the deployed code.

### The Test Harness

The suite runs the deployed binaries in `ckb-testtool`. Its `Context::verify_tx` executes the scripts but none of the other transaction checks of the node. So [every test verifies](scripts/tests/src/tests.rs) through `VerifyRealistic`, which adds two checks:

- **Node verifiers:** every transaction also goes through `ckb-verification` 1.2.1 under mainnet consensus, so a rejection test shows something only the scripts refuse. Its transaction verifier is unchanged from node v0.210.0. It covers capacity, the NervosDAO maximum withdraw and the DAO lock-size and output-data rules. Only checks that waiting satisfies (`since` and cellbase maturity) are left out. The few rejection tests whose point overlaps a node rule, such as a malformed DAO witness, name that rule explicitly. The chain replay below confirms the check rejects none of the transactions the real chain accepted.
- **Invariant oracle:** on every transaction the scripts accept, it re-derives the four invariants from the transaction and its resolved input cells, not from script code paths. It checks each one only when its owning script executes. It still parses cells with the same rules as the scripts (data lengths, the soft-capped conversion, the order encoding). So it catches a binary that differs from its source or a composition the scripts get wrong. It can't catch a rule that both the scripts and the oracle get wrong. A positive-control test feeds it one broken transaction per invariant, so a silenced oracle fails the suite.

Output locks never execute. So the creation-time blind spots that each invariant section documents stay outside the oracle. The oracle also never checks recipients (see [Authorization Boundary](#authorization-boundary)).

A fresh `cargo test --locked` run in `scripts/tests` passes with no failures. The exhaustive search and the chain replay are ignored long runs. The hand-written scenarios reproduce each attack and boundary case this report cites, including [transaction shapes replayed](scripts/tests/src/tests/replay.rs) from observed protocol flows. Four kinds of generated evidence follow.

### Structured Fuzzing

[The fuzzer](scripts/tests/src/tests/fuzz.rs) composes one to eight valid segments into a transaction, in a random valid order:

- iCKB deposit phase 1 and phase 2
- withdrawal phase 1 and phase 2 through `Owned Owner`
- limit order mint, match and melt, in both directions and from both mint and match state
- a plain NervosDAO deposit

Then it applies up to three mutations:

- capacities, data fields and data lengths
- swaps, duplicates and removals of cells
- lock and type swaps, including same-code scripts with non-empty args or under hash type `data2`
- header accumulated rates and header deps
- iCKB moved between outputs
- deposits rescaled together with their receipts
- order outputs nudged one unit off their match
- a DAO cell wrapped into an owned-owner pair
- out point re-pointing

Mutated cases the node would reject are dropped before the scripts run. An unmutated case that fails to verify fails the test, so the generator itself stays honest. Every generated header sits below block 10,000,000, so the DAO lock-size rule of the node never applies. So the harness accepts slightly more than a node would today. That widens what the oracle checks and hides nothing.

- **Volume:** 12 million cases. Of these, 9,033,755 passed the node check and ran through the scripts. The oracle checked every accepted one. None broke an invariant. Run as `ICKB_FUZZ_CASES=1000000` with seeds `0x5eed0021` to `0x5eed002c`, in `--release`.
- **Coverage:** the fuzzer triggered every validation error of the three scripts except `DuplicatedMaster` and `AttemptToChangeFulfilled`, which [Invariant 3](#invariant-3-limit-orders-keep-their-value) explains are unreachable. It also never triggered these syscall-level codes: `IndexOutOfBound` in all three scripts, `LengthNotEnough` in `iCKB Logic` and `Owned Owner`, `ItemMissing` in `Owned Owner` and `Limit Order`.
- **CI:** the default suite runs 100 fixed-seed cases. It checks that every segment kind verifies unmutated and is rejected once mutated. It prints the error codes it never reached.

### Exhaustive Small-Scope Search

Random cases can miss a narrow combination. So [the exhaustive search](scripts/tests/src/tests/fuzz/exhaustive.rs) enumerates every transaction of up to three inputs and three outputs, five cells in all, in every order. It draws them from a fixed set of 24 input and 33 output archetypes, plus one funding input and one change output. The archetypes sit where the checks switch:

- deposits at the 1,000 and 1,000,000 CKB bounds and one shannon past them
- the soft-cap boundary
- receipts and iCKB cells that match or miss by one
- owner distances that point right or wrong
- mint-state, half-filled and fulfilled orders
- look-alikes under same-code scripts with non-empty args

All amounts derive from one deposit block whose accumulated rate makes conversions round down.

- **Result:** 37,797,412 transactions. Of these, the node would reject 6,917,397 and the scripts reject 30,643,433. The scripts accept 236,582, none breaking an invariant.
- **Coverage:** every valid flow archetype is accepted somewhere, including phase-2 claims, withdrawals at the bounds and receipt redemption at the soft cap and at the maximum. These invalid archetypes are never accepted: an empty receipt, a receipt one shannon off its deposits, an owner two cells from its owned cell, receipts and owners of a sibling script. Deposits outside the bounds are accepted only where `iCKB Logic` doesn't run: the documented receiptless blind spot.
- **Cost:** about 2 hours as 20 parallel shards in `--release` on a 12-core laptop, `ICKB_EXHAUSTIVE_SHARD=i/20`. The default suite runs a small slice as a health check.

### Chain Replay

All 646 committed mainnet transactions that touch the iCKB scripts, up to block 20,978,907, replay through the oracle and the node check with no violation. So do all 9,808 on testnet, up to block 23,022,391: testnet runs the same deployment, with the same script hashes. [The fetcher](scripts/tests/mainnet/ickb_mainnet.py) checks every re-encoded transaction and header hash against the RPC. It also stores every cell dep, dep group members included. [The replay test](scripts/tests/src/tests/fuzz/mainnet.rs) runs on its fixtures without network.

The replay also executes each transaction: every script group, against the code cells the chain used. The chain accepted these transactions, so a rejection here would mean the harness runs scripts differently from a node. All 10,454 pass, across the iCKB scripts, `xUDT`, `NervosDAO` and every user lock in them.

### Mutation Testing

`cargo-mutants` 27.1.0 mutated `scripts/contracts` once, as a local report. It also made six mutants in the loader of the test harness (`scripts/tests/src/lib.rs`), which the counts below leave out. The suite loads prebuilt binaries. So a `$CARGO` wrapper rebuilt the three contracts for RISC-V from each mutated copy, with Homebrew clang compiling the C part of ckb-std. It then ran the whole suite against them with `CAPSULE_TEST_ENV=debug`. It skipped only the deployed-hash test, which no rebuild can pass. Against the unmutated rebuild, every other test passes.

| Crate         | Caught | Missed | Unviable |
| ------------- | ------ | ------ | -------- |
| `ickb_logic`  | 77     | 0      | 5        |
| `limit_order` | 87     | 0      | 14       |
| `owned_owner` | 19     | 0      | 2        |
| `utils`       | 46     | 2      | 30       |

Unviable mutants don't compile: mostly constant and type positions where the replacement has no valid value. One caught mutant is equivalent: the soft cap `>` to `>=` in `deposit_to_ickb` gives the same value at the boundary. It fails only the check that the copies in the vector generator still match the contract sources. Two caught mutants needed tests of their own:

- `>` to `>=` on the `u64::MAX` output ceiling in `check_output`, caught by a [boundary test that runs `iCKB Logic`](scripts/tests/src/tests/ickb_logic/xudt_amount_limits.rs).
- `has_dao_type` returning `Ok(true)`, caught by [request-shaped data under a non-DAO type](scripts/tests/src/tests/owned_owner/script_misuse.rs).

The two survivors are equivalent: `||` to `&&` in `from_hex`, which only validates compile-time constants that are already valid. No test can tell them apart.

How much does a clean fuzz run prove? The fuzz test alone, at its 100-case CI budget, catches 196 of the 229 caught mutants. Running 20,000 fuzz cases against each of the other 33 detects 22. For 14 the oracle flags an accepted invariant break. For 8 a valid base case is rejected. The 11 it misses:

- 10 change no invariant: the soft cap `>` to `>=` (equal at the boundary), three `has_empty_args` mutants (an args guard), two direction checks that only admit donations to an order, the sign of `ckb_unoccupied` (only stricter), the two `u64::MAX` ceiling mutants (a script guard, covered by the test above) and deleting the check against orders with both ratios empty (such an order can be minted and melted, but no fill of it can pass `validate`).
- 1 is a remaining blind spot: `is_deposit_data` returning `true`. Its accepted transactions need an iCKB-locked withdrawal request spent with exactly its value burned, which takes several coordinated changes. The [iCKB-locked request test](scripts/tests/src/tests/ickb_logic/creation_blind_spots.rs) catches it.

### Instruction Coverage

The checks above found no accepted transaction that breaks a rule. They don't show how much of the code on chain those checks really run. A binary also holds code that isn't in the contract source: the compiler and the libraries add their own. So [the coverage tooling](scripts/tests/coverage/README.md) records every machine instruction the three deployed binaries execute during a run. It works on the deployed bytes themselves: the copies that carry function names equal them byte for byte in every loaded section.

The record covers the default suite, 900,000 more fuzz cases (678,376 reached the scripts), two slices of the exhaustive search (77,203 transactions run) and the replay of both chains.

It showed code that a transaction can reach and that no run had ever executed:

- **Map splits:** each script keeps its bookkeeping in a sorted map. The map stores its entries in nodes of 11 and splits a node that fills up. No test, fuzz case or chain transaction gave `iCKB Logic` 12 different deposit amounts, so its split code had never run. `Limit Order` had split nodes only in testnet transactions.
- **Long scripts:** every script reads its own script and each output lock to check the args. A script of up to 256 bytes fits one read. A longer one takes a second read into heap memory, which no run had needed.
- **Division correction:** the iCKB conversion divides a 128-bit number. The division routine guesses the result, then corrects the guess when needed. One correction had never run. At current rates it needs an amount below 77 CKB, which only a receiptless deposit holds.
- **Large sums:** the sums of `iCKB Logic` have 128 bits. Their upper half only comes into play past 2^64 units, more than any chain state holds.
- **Out of memory:** no run had used up the memory of a script.

[The reach tests](scripts/tests/src/tests/fuzz/reach.rs) now run each of these and check the verdict both ways: accepted when right, refused when one entry among many is off by one unit. An independent pass over what was left found three more spots, now tested the same way: a split with the new entry in the middle of a node, a script reading its own long args, `Owned Owner` running out of memory.

With these tests, the runs execute:

- **iCKB Logic:** 8,995 of 14,033 instructions (64.1%)
- **Limit Order:** 6,259 of 8,642 instructions (72.4%)
- **Owned Owner:** 4,562 of 6,753 instructions (67.6%)

The default suite alone reaches almost the same: 64.1%, 72.4% and 67.2%.

Between a quarter and a third of each binary still never runs. Every instruction of it falls in one of three groups. The counts are for `iCKB Logic`, `Limit Order` and `Owned Owner`, in that order:

- **Message formatting** (3,443, 1,006 and 1,040 instructions): functions that are never entered, almost all of them code that builds panic messages. The release build prints no message ([`debug!`](https://github.com/nervosnetwork/ckb-std/blob/a6b9b66570e6f7f5c41a76249db6fb663011cf3c/src/debug.rs) compiles to nothing), so nothing calls them.
- **Stops** (708, 668 and 581): paths that end by stopping the script. They are overflow checks that the [Arithmetic Bounds](#arithmetic-bounds) rule out, checks inside the standard library that its own logic guarantees, out-of-memory exits other than the ones the tests trigger.
- **The rest** (887, 709 and 570): read piece by piece.

The rest falls into these groups:

- **Failures a node never returns:** the handling of a field load that fails with anything but the end of the list, of a wrong length for an out point or a header field, of an unknown error code.
- **Malformed scripts:** the handling of a script whose encoding is broken. Each such path refuses the transaction.
- **Empty transactions:** the case of a transaction with no inputs or no outputs, which the [node refuses](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/verification/src/transaction_verifier.rs#L383-L406).
- **One fixed input:** hashing and script building run once per execution, always on the same bytes (the `xUDT` script of iCKB). Their paths for other lengths never run.
- **Upper words that stay zero:** the top word of the 256-bit sums in `Limit Order`, which stay below 2^192. Also the upper words of its one subtraction, which works on 64-bit amounts.
- **Library variants:** paths of the memory copy routines for layouts these programs never produce. Allocator paths for a heap that isn't zero-filled, for a single block above 1 MiB, for more than 64 small blocks alive at once. Division paths for a divisor above 64 bits.
- **Defensive errors:** `DuplicatedMaster` and `AttemptToChangeFulfilled` in `Limit Order` (see [Invariant 3](#invariant-3-limit-orders-keep-their-value)).

No never-executed instruction was found that a transaction can reach and that would change a verdict.

### Compiled-In Dependencies

The compiler links libraries into each binary: `ckb-std` 0.15.3 with its C part, the `buddy-alloc` 0.5.1 allocator, `molecule` 0.7.5 with `ckb-gen-types` 0.112.1, `blake2b-ref` 0.3.1, `primitive-types` 0.12.2 over `uint` 0.9.5, plus the parts of the Rust standard library the scripts call. A defect in any of them would be a defect in the contracts.

So the later releases of each were read for fixes in code the scripts use. Where a fix looked relevant, the deployed bytes were inspected:

- **ckb-std:** [the 72 commits after v0.15.3](https://github.com/nervosnetwork/ckb-std/compare/a6b9b66570e6f7f5c41a76249db6fb663011cf3c...4657ce2ace29df60d97862800da7c2d998b70d1d) don't change how the code the scripts call behaves: the syscall wrappers, the loaders, the entry point, the allocator setup. One commit documents that loading a large value can run out of memory, which the long-lock limit in [Liveness](#liveness) measures.
- **Memory routines:** four small C routines copy, fill and compare memory, with the same bytes in all three binaries. Run under an emulator against a byte-by-byte reference, they agree on 1,935,168 calls: every alignment, all short lengths plus larger ones, every overlap in both directions. A copy with one instruction disabled fails 14,818 of them, so the check does notice a broken routine.
- **buddy-alloc:** version 0.6.0 rewrote its free-list writes after [a report](https://github.com/jjyr/buddy-alloc/issues/16) that an optimized build lost one of them. A lost write could hand out the same memory twice. The deployed bytes don't have that defect: the disassembled allocator of all three binaries performs every write.
- **uint:** [its changelog](https://github.com/paritytech/parity-common/blob/d2817a6d9118ee985805c37b2a44c0f2c9ae3e93/uint/CHANGELOG.md) lists no arithmetic fix after 0.9.5.
- **molecule and ckb-gen-types:** the later fixes sit in code these versions don't have (a lazy reader) or the scripts never call (unpacking a `Bool`).
- **blake2b-ref:** the hash routine in the deployed `iCKB Logic` gives the reference BLAKE2b result on 512 inputs under the same emulator. The script only ever hashes one value with it: the `xUDT` script of iCKB.
- **Sorted map:** the scripts only insert into their maps, then consume them. They use the global allocator. A panic ends the script. The fixes found after Rust 1.74.0 concern other ground: `append`, cursors, custom allocators and panic safety.

### Compiler Defects

The contracts were compiled once, with rustc 1.74.0 and its LLVM 17. A defect of that compiler that produces wrong code matters for as long as the contracts live, even when it is found years later. So an independent audit went through the known reports of wrong code: 255 for Rust and 1,940 for LLVM.

It judged each one from the 1.74.0 sources, not from the version a report names. Where it could, it checked exactly:

- **LLVM defects:** it rebuilt that LLVM and showed that it reproduces every compiled object of the contracts byte for byte. Then it applied each later fix that fits that LLVM and compared the functions that end up in the deployed binaries.
- **Rust defects:** it scanned the intermediate code of every function before and after the faulty step, on a rebuild of the standard library made with the settings of the shipped one.

The result, for the defects that are in 1.74.0:

- **Present, never fired:** [rust#158206](https://github.com/rust-lang/rust/issues/158206), [rust#150904](https://github.com/rust-lang/rust/issues/150904), [rust#158231](https://github.com/rust-lang/rust/issues/158231), [rust#118328](https://github.com/rust-lang/rust/issues/118328), [rust#137936](https://github.com/rust-lang/rust/issues/137936), [rust#146423](https://github.com/rust-lang/rust/issues/146423), [rust#162997](https://github.com/rust-lang/rust/issues/162997), [llvm#183906](https://github.com/llvm/llvm-project/issues/183906), [llvm#63763](https://github.com/llvm/llvm-project/issues/63763), [llvm#97702](https://github.com/llvm/llvm-project/issues/97702), [llvm#91178](https://github.com/llvm/llvm-project/issues/91178), [llvm#218569](https://github.com/llvm/llvm-project/issues/218569), [llvm#233085](https://github.com/llvm/llvm-project/issues/233085), [llvm#215533](https://github.com/llvm/llvm-project/issues/215533), [llvm#236396](https://github.com/llvm/llvm-project/issues/236396), [llvm#115976](https://github.com/llvm/llvm-project/issues/115976), [llvm#236347](https://github.com/llvm/llvm-project/issues/236347), [rust#164237](https://github.com/rust-lang/rust/issues/164237). With the fix applied, or the faulty step watched, no deployed function changes.
- **Present, out of reach:** [rust#137646](https://github.com/rust-lang/rust/issues/137646), [rust#163856](https://github.com/rust-lang/rust/issues/163856), [rust#131834](https://github.com/rust-lang/rust/issues/131834), [rust#119160](https://github.com/rust-lang/rust/issues/119160) and [rust#163777](https://github.com/rust-lang/rust/issues/163777) need code shapes the deployed functions don't have, such as a virtual call that really runs. [rust#141101](https://github.com/rust-lang/rust/issues/141101) and [rust#132898](https://github.com/rust-lang/rust/issues/132898) don't change the compiled code.
- **Present, fired, same behavior:** the fix for [llvm#84200](https://github.com/llvm/llvm-project/issues/84200) changes two deployed functions: the same instructions, in another order. The fix for [llvm#84653](https://github.com/llvm/llvm-project/issues/84653) changes seven: each still computes the same condition from the same values. [llvm#68683](https://github.com/llvm/llvm-project/issues/68683) writes a value into a field that is left undefined on an error path.
- **Present, not shown to fire:** [rust#137668](https://github.com/rust-lang/rust/issues/137668), [rust#164599](https://github.com/rust-lang/rust/issues/164599) and [rust#145956](https://github.com/rust-lang/rust/issues/145956), without an exhaustive search.

About 1,400 more reports don't apply: other targets, floating point, link-time optimization, steps this build doesn't run.

The audit has limits. It couldn't decide 348 LLVM reports and 17 Rust ones: most are open reports without a fix, or fixes that don't fit LLVM 17. It counts a function as deployed when the deployed code refers to it directly. It doesn't cover the four memory routines, which another compiler built (see [Compiled-In Dependencies](#compiled-in-dependencies)). Its check of the 128-bit division is partly approximate. The tests cover what an audit can't: they run the compiled bytes, as [Instruction Coverage](#instruction-coverage) shows, the division on every branch.

### Arithmetic Bounds

Can a sum or a product overflow? The scripts are built with overflow checks (`overflow-checks = true` in the release profile of [`scripts/Cargo.toml`](scripts/Cargo.toml)), so an overflow stops the script instead of wrapping around. The deployed bytes carry those checks: [a sum past 2^128 stops the script](scripts/tests/src/tests/fuzz/reach.rs), in cases built so that a wrapped sum would balance.

No value a chain can hold comes near an overflow anyway:

- **iCKB conversion:** `amount * 10^16` takes at most 118 bits for any 64-bit amount. Its division by the accumulated rate gives the natively computed result on [every branch of the division routine](scripts/tests/src/tests/fuzz/reach.rs), the two corrections included.
- **iCKB sums:** every header counts all CKB issued in 64 bits ([RFC 0023](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0023-dao-deposit-withdraw/0023-dao-deposit-withdraw.md#calculation)), so all pool deposits together are worth less than 2^64 iCKB. Supply and receipts never exceed them (see [From One Transaction to the Whole Chain](#from-one-transaction-to-the-whole-chain)). Each iCKB output is capped at `u64::MAX`. So every 128-bit sum stays far below its limit.
- **Limit Order:** CKB amounts and ratio multipliers have 64 bits, UDT amounts 128. The largest sum the script forms is `(2^128 - 1) * (2^64 - 1) + 2^64 * (2^64 - 1)`, below 2^192. So its 256-bit operations never overflow, for any token. [Fills at the limits of every operand](scripts/tests/src/tests/fuzz/reach.rs) agree with a price computed by one division: the fill passes at the price rounded up and fails one unit below.

## Invariant 1: No iCKB From Nothing

When `iCKB Logic` runs, input iCKB plus the value of redeemed receipts equals output iCKB plus the value of spent pool deposits. When it doesn't run, iCKB supply never grows.

### The Rule

[`entry.rs:32`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L32) enforces the balance equation:

```rust
if in_udt_ickb + in_receipts_ickb != out_udt_ickb + in_deposits_ickb {
    return Err(Error::AmountMismatch);
}
```

Every flow is an instance of it: deposit phase 1 is `0 + 0 == 0 + 0`, deposit phase 2 is `0 + receipt_value == out_udt + 0`, a withdrawal is `in_udt + 0 == out_udt + deposit_value`. A mixed transaction combines them. Four properties make the equation mean what it says:

- **Cell classification:** [`celltype.rs:60-82`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L60-L82) classifies every cell in the transaction by its lock and type script hashes, plus the DAO data shape when the DAO hash is present.
- **One conversion:** [`deposit_to_ickb`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L71-L84) values receipts and deposits alike.
- **Exact script identity:** [`has_empty_args`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L14-L36) requires empty args on the running script and on [every output lock](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L29-L31) with the same code hash and hash type. Output types need no check, because they execute and check themselves.
- **No silent wrapping:** [`overflow-checks = true`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/Cargo.toml#L6) makes any overflow abort the script. Each output iCKB amount must also be at most `u64::MAX` ([`entry.rs:123-125`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L123-L125)).

| Lock                               | Type                       | Classification         | Line                                                                                                                                    |
| ---------------------------------- | -------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| iCKB Logic                         | DAO deposit (8 zero bytes) | `Deposit`              | [L69](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L69) |
| iCKB Logic                         | anything else              | `ScriptMisuse` (error) | [L72](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L72) |
| other                              | iCKB Logic                 | `Receipt`              | [L75](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L75) |
| other                              | iCKB xUDT                  | `Udt`                  | [L78](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L78) |
| DAO deposit (8 zero bytes) as lock | any                        | `ScriptMisuse` (error) | [L62](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L62) |
| iCKB xUDT as lock                  | any                        | `ScriptMisuse` (error) | [L63](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L63) |
| other                              | other                      | `Unknown` (ignored)    | [L81](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L81) |

`ScriptType::None` is only synthesized for [missing type scripts](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L48-L50). [`script_type()`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L95-L112) never returns `None` for locks, so the [`(ScriptType::None, _)`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L64) arm is unreachable.

The conversion:

```rust
let ickb_amount = amount * ar_0 / ar_m;
if ickb_amount > ICKB_SOFT_CAP_PER_DEPOSIT {
    return Ok(ickb_amount - (ickb_amount - ICKB_SOFT_CAP_PER_DEPOSIT) / 10);
}
```

`amount` is a `u64` and the product fits in `u128` (about `1.8e19 * 1e16 = 1.8e35 < 3.4e38`). Integer division loses at most one shannon per operation.

### Attacks and Why They Fail

- **(1A) Direct xUDT minting bypass:** blocked. Owner mode has only the two live routes of [xUDT Owner Mode](#xudt-owner-mode). Both run `iCKB Logic`, so the balance equation applies. The [output-witness and input-witness owner-script fallback tests](scripts/tests/src/tests/ickb_logic/xudt_owner_witness_fallback.rs) confirm that the witness route fails under `xUDT` amount checks. `xudt_rce.c` discards the result of that route, so the tests can't show why it failed. The source argument above establishes why.
- **(1C) Inflate a receipt value through the accumulated rate of its block:** blocked. [`extract_accumulated_rate`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L54-L61) calls `load_header`, which reads the block hash from [`CellMeta.transaction_info.block_hash`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/syscalls/load_header.rs#L56-L60). It succeeds only when that block is also in [`header_deps`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/syscalls/load_header.rs#L61-L66). The transaction creator can't override that link.
- **(2A) Withdraw a deposit without burning iCKB:** blocked. With no iCKB or receipts on input, `0 + 0 != 0 + deposit_value` fails with `AmountMismatch`.
- **(2B) Spend a deposit through NervosDAO without `iCKB Logic`:** blocked. Deposits use `iCKB Logic` as their lock. CKB always executes input locks (the [lock-script rule](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#lock-script)).
- **(2C) Read value through a cell dep:** no attack vector. A [cell dep](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#code-locating) stays live and runs none of its scripts.
- **(3A) Deposit oversized, then withdraw the same deposit:** no arbitrage. The soft-cap fee at minting and the discount at withdrawal come from the same `deposit_to_ickb` call with the same inputs ([`entry.rs:58-59`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L58-L59) for receipts, [`entry.rs:52`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L52) for deposits). So the minted and burned amounts are equal.
- **(3B) Cherry-pick the cheapest deposits:** no mispricing. [`deposit_to_ickb`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L76) computes `amount * AR_0 / AR_m`. So deposits with a higher `AR_m` cost less nominal iCKB, because the formula values them lower. That matches the [maximum withdraw](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0023-dao-deposit-withdraw/0023-dao-deposit-withdraw.md#calculation) of RFC 0023 and the [exchange rate](https://github.com/ickb/whitepaper/blob/master/README.md#ickbckb-exchange-rate-calculation) of the whitepaper. The formula values only unoccupied capacity, while the phase 2 claim also returns the occupied capacity of the deposit (82 CKB for a standard deposit cell). So below the soft cap, for each iCKB burned a withdrawer receives an extra 82 CKB divided by the iCKB value of the deposit, on top of the `AR_n / AR_0` every deposit returns: 0.082 CKB per iCKB from a deposit worth 1,000 iCKB, against 0.00082 CKB from one worth 100k. Withdrawing the deposits with the smallest iCKB value first is optimal. That value depends on the accumulated rate of the deposit as well as on its unoccupied capacity. Oversized deposits also pay more per iCKB, through the soft-cap discount (about 9.9% at 1M CKB at the genesis rate). This is the [penalty on small deposits](https://github.com/ickb/whitepaper/blob/master/README.md#deposit-phase-1) the whitepaper intends: the depositor pays it while the withdrawer collects it ([small-versus-standard withdrawal test](scripts/tests/src/tests/ickb_logic/economic_precision.rs)).
- **(3C) Accumulate rounding errors over many small deposits:** blocked by exact shannon precision. The [rounding mint, withdrawal and non-genesis-AR soft-cap boundary tests](scripts/tests/src/tests/ickb_logic/economic_precision.rs) show that whole-CKB or rounded claims fail with `AmountMismatch` unless the exact value and operation order are used: integer AR normalization comes before the per-deposit haircut.
- **(4C) A cell with an `iCKB Logic` lock and a non-DAO type:** can't be spent. Output locks don't run, so it can be created. When it is spent, `iCKB Logic` rejects `(ScriptType::IckbLogic, _)` with `ScriptMisuse` ([`celltype.rs:72`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L72)), as the [lock-only non-DAO output test](scripts/tests/src/tests/ickb_logic/creation_blind_spots.rs) shows.
- **(4D) A NervosDAO withdrawal request counted as a deposit:** blocked. [`is_deposit_data`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/dao.rs#L17-L22) requires exactly 8 zero bytes, while a request stores its deposit block number. So a request classifies as `Unknown` under any other lock. Under an `iCKB Logic` lock it fails with `ScriptMisuse` ([iCKB-locked request test](scripts/tests/src/tests/ickb_logic/creation_blind_spots.rs)).
- **(5C) A non-empty-args `iCKB Logic` variant:** blocked. Different args give a different script hash, so [`script_type()`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L95-L112) doesn't recognize the cell. Run as its own group, it fails `has_empty_args`. The poisoning regressions in [phase2_sibling_classification.rs](scripts/tests/src/tests/ickb_logic/phase2_sibling_classification.rs) cover output and input siblings. The same code under hash type `data2` with empty args passes `has_empty_args`, but it also has a different hash. So it runs as an independent copy of the protocol with its own token. The fuzzer mixes such copies into its transactions.
- **(6B) `u128` overflow in the balance equation:** blocked by `overflow-checks` and practical bounds.
- **(6C) Division by zero in `deposit_to_ickb`:** impossible. The [RFC 0023 accumulated-rate rule](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0023-dao-deposit-withdraw/0023-dao-deposit-withdraw.md#calculation) sets `AR_0 = 10 ^ 16` and `AR_i = AR_{i-1} + floor(AR_{i-1} * s_i / C_{i-1})`, so every `AR_m` is non-zero.
- **(6D) Underflow in `extract_unused_capacity`:** impossible. The node enforces `capacity >= occupied_capacity` for every cell. The suite runs that verifier on every accepted transaction. So the subtraction at [`utils.rs:48`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L48) is safe.
- **(6G) Spend a receipt and its deposit from the same block together:** no profit. Both read the same accumulated rate, so `receipt_value == deposit_value`. The balance equation then forces `out_udt = 0`. The [same-block receipt and deposit test](scripts/tests/src/tests/ickb_logic/mixed_flow_composition.rs) shows that minting 0 passes while minting 1 fails with `AmountMismatch`.

### Limits

- **Receiptless deposits:** CKB doesn't execute output locks. So a DAO-shaped output under the `iCKB Logic` lock can be created without a receipt. [`cell_type_iter`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/celltype.rs#L68-L75) later counts it as a pool deposit, with no provenance check ([receiptless deposit-admission test](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs)). Its creator receives no iCKB, so it only adds value that no minted iCKB matches. One accepted path pairs a separately provided split receipt with a self-funded receiptless aggregate deposit. The contract mints only the soft-cap valuation delta, which leaves the user worse off than redeeming the receipt normally. In the tested case, a receipt for two 100k CKB deposits plus a separately funded 200k CKB aggregate yield 10k iCKB plus the CKB of the aggregate back, instead of the 200k iCKB of the receipt. The 190k iCKB of value left in the pool is effectively burned. The [delta-only spread test](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs) and the [oversized spread test](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs) show the accepted spread. The [deposit-alone spread block test](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs) blocks the deposit-alone variant. The [mixed phase1-phase2 soft-cap test](scripts/tests/src/tests/ickb_logic/soft_cap.rs) shows that fresh phase-1 deposits in the same transaction don't bypass the soft cap. The [continuous trajectory](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs) goes from split-deposit receipt creation through receiptless creation, spread realization and DAO phase 2. It keeps the transaction hash and index of each verified output, with no theft of third-party principal or duplicated recovery.
- **Prefix parsing:** trailing bytes in receipt and `xUDT` data are read as a prefix, while truncated encodings are rejected ([receipt trailing-bytes and truncated receipt tests](scripts/tests/src/tests/ickb_logic/receipt_encoding.rs), [trailing and short `xUDT` output-data tests](scripts/tests/src/tests/ickb_logic/phase2_xudt_output_data.rs)).
- **Recipients:** the equation never looks at who receives the output iCKB (see [Authorization Boundary](#authorization-boundary)).

## Invariant 2: Receipts Match Deposits

Every receipt created counts pool deposits created in the same transaction, one for one per amount, each within 1,000 to 1,000,000 CKB.

### The Rule

[`check_output`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L86-L140) keys a `BTreeMap<u64, Accounting>` by amount:

- each output deposit adds one under its [unused capacity](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/utils.rs#L47-L49);
- each output receipt adds its quantity under its [deposit amount](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/utils.rs#L18-L40);
- every amount must end with `deposited == receipted` ([`entry.rs:132-137`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L132-L137)).

It also requires each deposit created with a receipt to hold `1000 CKB <= unused capacity <= 1M CKB` ([`entry.rs:101-106`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L101-L106), bounds at [`constants.rs:5-6`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/constants.rs#L5-L6)). Each receipt must count at least one deposit ([`entry.rs:114-116`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L114-L116)). A receipt and its deposits sit in the same block, so at redemption the receipt is worth what its deposits are worth.

### Attacks and Why They Fail

- **(1B) A receipt that claims more deposits than exist:** blocked. The [over-counted receipt test](scripts/tests/src/tests/ickb_logic/receipt_matching.rs) shows a receipt can't claim more same-sized deposits than the transaction creates.
- **(1D) A receipt never backed by deposits:** blocked. The type of a receipt is `iCKB Logic`, so creating it runs the accounting above. The [UTXO model](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#value-storage) of CKB keeps the created cell immutable.
- **(4A) A receipt for zero deposits:** blocked. The [zero-quantity receipt test](scripts/tests/src/tests/ickb_logic/receipt_matching.rs) shows its creation fails, so no such receipt can exist as an input.
- **(4B) A receipt amount without a matching deposit:** blocked. The [unmatched receipt-amount test](scripts/tests/src/tests/ickb_logic/receipt_matching.rs) shows it fails with `ReceiptMismatch`.
- **(4E) Inflate the unused capacity of a deposit:** blocked. CKB requires [occupied capacity to fit within cell capacity](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#cell-data). `extract_unused_capacity` subtracts the occupied capacity the host reports, which the deposit structure fixes (`c_o = 82 CKB` for the standard shape in the [exchange-rate calculation](https://github.com/ickb/whitepaper/blob/master/README.md#ickbckb-exchange-rate-calculation) of the whitepaper).
- **(4F) Reset the age of a deposit by re-depositing at the same index:** blocked ([whitepaper#18](https://github.com/ickb/whitepaper/issues/18)). When a DAO deposit is consumed, [`validate_withdrawing_cell`](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L337-L389) requires a withdrawal request at the same index, storing the deposit block number. The zero data of a fresh deposit fails [that comparison](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L382-L385). The [same-index reset test](scripts/tests/src/tests/ickb_logic/mixed_flow_composition.rs) withdraws and re-deposits in one transaction. It verifies with the request at the deposit index and fails with error `-20` when they swap. The receiptless aggregate-deposit variant is the separate limit of [Invariant 1](#invariant-1-no-ickb-from-nothing).
- **(6A) Boundary values:** correct. The minimum check uses `<` ([`entry.rs:101`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L101)) and the maximum `>` ([`entry.rs:104`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L104)). So exactly 1,000 and exactly 1,000,000 CKB are both allowed ([deposit_bounds.rs](scripts/tests/src/tests/ickb_logic/deposit_bounds.rs)).
- **A cell that is both an order and a receipt:** each script reads a cell locked by `limit_order` and typed by `iCKB Logic` through its own prefix of the same data. Matching it for one shannon raises the receipt quantity from one to two. So the [receipt-typed order test](scripts/tests/src/tests/ickb_logic/mixed_flow_composition.rs) verifies only when the match creates the two deposits the continued receipt counts and mints exactly the redeemed value. With one of them missing, it fails with `ReceiptMismatch`.
- **(6H) Someone withdraws a deposit before its receipt is redeemed:** pool accounting stays balanced. The receipt value was fixed at creation, so its holder still redeems it normally. The withdrawer paid [`deposit_to_ickb`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/ickb_logic/src/entry.rs#L71-L84) for the deposit.

### Limits

The bounds are part of the output check. It runs only when the creating transaction also executes `iCKB Logic`, as it does with a receipt. A receiptless deposit is created without that check, so the pool can hold deposits of any size. One below the minimum is still withdrawn at its exact iCKB value ([receiptless below-the-minimum test](scripts/tests/src/tests/ickb_logic/receiptless_deposit_blind_spot.rs)). Its creator receives no iCKB. So withdrawing it costs its full iCKB value, while the withdrawer gains only the occupied capacity of the deposit. Integrations must not assume pool deposits lie within the bounds.

## Invariant 3: Limit Orders Keep Their Value

An order spent without its master continues into exactly one order with the same master and info. Its value at its own ratio doesn't decrease. Its fill respects the minimum match. Masters move only with their orders.

### The Rule

[`Limit Order`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs) supports three operations: `Mint` (create order plus master), `Match` (partial or full fill) and `Melt` (destroy order plus master). Orders store exchange ratios (`ckb_to_udt`, `udt_to_ckb`) and a minimum match size.

**Value conservation:** [`entry.rs:103-105`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L103-L105) rejects a match that lowers the order value. It computes in [`C256`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/utils/src/c256.rs), a checked `U256`, so it can't overflow:

```rust
if i.ckb * ckb_mul + i.udt * udt_mul > o.ckb * ckb_mul + o.udt * udt_mul
```

**Concave ratios:** [`entry.rs:233-235`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L233-L235) makes sure a round trip through both ratios never loses value for the maker:

```rust
if c2u.ckb_mul * u2c.udt_mul < c2u.udt_mul * u2c.ckb_mul
```

**Minimum match:** [`entry.rs:108-129`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L108-L129) prevents dust-level partial matches. A fill that completes the order is exempt.

**Strict data length:** when `limit_order` executes, order data must be exactly `UDT_SIZE + ORDER_SIZE` (89) bytes ([`entry.rs:166`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L166)).

### Attacks and Why They Fail

- **(5E) Lower the value of an order in a match:** blocked by the conservation check above.
- **Re-wire real orders to other masters:** blocked. Differing mint capacities and differing match progress both block cross-wiring ([distinct mint-capacity](scripts/tests/src/tests/limit_order/crosswire_blockers.rs) and [distinct match-progress](scripts/tests/src/tests/limit_order/crosswire_blockers.rs) tests). Real orders also fail to cross-wire whether their checked info matches or differs ([same-info](scripts/tests/src/tests/limit_order/crosswire_blockers.rs) and [different-info mainnet](scripts/tests/src/tests/limit_order/crosswire_blockers.rs) tests).
- **Harmless pairings:** permuting two indistinguishable matched orders strands neither master. Both continuations stay redeemable by the master encoded in their data, also with masters protected by `secp256k1_blake160_sighash_all` ([cloned-order permutation test](scripts/tests/src/tests/limit_order/crosswire_live_match.rs)). A mint can pair any of its order outputs with any of its master outputs, but the script enforces one order per master ([mint pairing rule test](scripts/tests/src/tests/limit_order/crosswire_mint_creation.rs)). The minter creates and controls all of them, so a crossed pairing is a choice the minter made ([mint pairing test](scripts/tests/src/tests/limit_order/crosswire_mint_creation.rs)).
- **Unreachable errors:** a terminal CKB -> UDT fill passes at exact occupied capacity. Continuing a fulfilled single-direction order fails at the outer `InvalidMatch` check (a dual-ratio order may still move in its other direction, as designed). A lock-only order without a UDT type returns `MissingUdtType`. Two defensive errors can't be reached by consensus-valid cells. `AttemptToChangeFulfilled` can't, because on the CKB -> UDT side a continuation shares the lock, type and data length of the order, so it can't hold less CKB than a fulfilled order. On the UDT -> CKB side, a fulfilled order has no UDT left to decrease. The [vector replay](scripts/tests/src/tests/protocol_vectors.rs) skips the two vectors that assume such a shrinking CKB -> UDT cell, whose two sides imply different occupied capacities. It also asserts which two they are. [`DuplicatedMaster`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L42-L45) can't either: an input master is keyed by its out point, an output master by its output index. No two cells share either.

### Limits: LO-01

**(5F) Confusion attack:** the one live issue. CKB builds lock groups only from input locks ([`types.rs:642-666`](https://github.com/nervosnetwork/ckb/blob/2592ddf0502cd4adfe886db893cccc866db3c60f/script/src/types.rs#L642-L666)). An order cell is the lock-only `(true, false)` case in [`limit_order/entry.rs:48-56`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/limit_order/src/entry.rs#L48-L56). So order outputs can be created with arbitrary master out points without running `limit_order`. The executed cases:

- Phantom orders can be created without master validation: the [phantom mint creation test](scripts/tests/src/tests/limit_order/phantom_lineage.rs).
- That phantom lineage can then enter and keep advancing through fake match state without a real master: the [phantom mint continuation test](scripts/tests/src/tests/limit_order/phantom_lineage.rs) and the [fake match lineage continuation test](scripts/tests/src/tests/limit_order/fake_match_lineage.rs).
- A fake order for a foreign token, with unrelated pricing and order information, can melt against a real master and strand the real order: the [real-order stranding test](scripts/tests/src/tests/limit_order/fake_match_lineage.rs).

The issue is [documented in the whitepaper](https://github.com/ickb/whitepaper/blob/master/README.md#confusion-attack-on-limit-order). Its mitigation is off-chain: compare each live order with the mint order created beside its master. Keep only orders with the same parameters, at least its value and at least its progress (a dual-ratio order has no progress, so only value counts). Then prefer the most progress, or the most value for a dual-ratio order. The iCKB Stack does this for the single-direction orders it handles ([`findOrigin`](https://github.com/ickb/stack/blob/c8edfa8276108472612e2f1965e18ff85b0da73f/sdk/src/order/order.ts#L228-L269), [`isValid` and `resolve`](https://github.com/ickb/stack/blob/c8edfa8276108472612e2f1965e18ff85b0da73f/sdk/src/order/cells.ts#L172-L210)).

## Invariant 4: Withdrawal Claims Stay One to One

In inputs and in outputs separately, each owned cell is a NervosDAO withdrawal request paired with exactly one owner cell.

### The Rule

[`Owned Owner`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/owned_owner/src/entry.rs) pairs owned cells (NervosDAO withdrawal requests) with owner cells through a `MetaPoint`. The `MetaPoint` comes from the `owned_distance` field of the owner cell: at mint the owner stores a signed distance, so `owned_index == owner_index + owned_distance`. At melt both cells are consumed together. For inputs and outputs separately:

- Each metapoint must have exactly `owned == 1` and `owner == 1` ([`entry.rs:57-60`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/owned_owner/src/entry.rs#L57-L60)).
- Owned cells must be NervosDAO withdrawal requests: DAO type and non-zero data ([`entry.rs:45-47`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/owned_owner/src/entry.rs#L45-L47)).
- A cell using the script as both lock and type is rejected ([`entry.rs:53`](https://github.com/ickb/contracts/blob/454cfa966052a621c4e8b67001718c29ee8191a2/scripts/contracts/owned_owner/src/entry.rs#L53)).

### Attacks and Why They Fail

- **(5D) An orphaned owner cell:** blocked when `Owned Owner` executes. The [orphan-owner rejection test](scripts/tests/src/tests/owned_owner/pair_formation.rs) shows an orphan owner is rejected right away. The [two-owner](scripts/tests/src/tests/owned_owner/pair_formation.rs) and [two-owned](scripts/tests/src/tests/owned_owner/pair_formation.rs) mismatch tests reject the count mismatches.
- **Rotate a live claim into a fresh pair, or cross-wire a DAO-constrained batch:** blocked. The [fresh-pair rotation block test](scripts/tests/src/tests/owned_owner/live_claim_rotation.rs), the [new-pair rotation block test](scripts/tests/src/tests/owned_owner/live_claim_rotation.rs) and the [DAO index-rule crosswire block test](scripts/tests/src/tests/owned_owner/dao_crosswiring.rs) show that block.
- **Signed lifecycle:** a [secp-protected sparse pair](scripts/tests/src/tests/owned_owner/positive_distance_claims.rs) completes phase 1 with a noncontiguous signed input group, then the signed owner spend in DAO phase 2.

### Limits

- **Weak-lock cross-wiring:** when the input locks of a phase-1 withdrawal don't bind its outputs, whoever assembles it can cross-wire the owner cells (see [Authorization Boundary](#authorization-boundary)). Later DAO claims are then reassigned across mixed foreign-plus-iCKB batches or fully iCKB batches ([mixed foreign-plus-iCKB](scripts/tests/src/tests/owned_owner/mixed_asset_crosswiring.rs), [two-way](scripts/tests/src/tests/owned_owner/dao_crosswiring.rs) and [three-way](scripts/tests/src/tests/owned_owner/dao_crosswiring.rs) weak-lock crosswire tests). Under the current strong-lock deployment assumption, this isn't a present finding.
- **Lock-only look-alikes:** a cell that uses `Owned Owner` only as its lock doesn't run it at creation. So unless another cell of the transaction runs `Owned Owner`, creation accepts it. It then fails when spent ([lock-only non-DAO](scripts/tests/src/tests/owned_owner/script_misuse.rs) and [lock-only DAO-deposit](scripts/tests/src/tests/owned_owner/script_misuse.rs) look-alike tests). The same goes for lock-only orphan requests ([orphan withdrawal-request dead-state test](scripts/tests/src/tests/owned_owner/pair_formation.rs)). Each strands only what its creator built that way.

## From One Transaction to the Whole Chain

The invariants are per transaction. They also bound the whole chain. Let the pool surplus `P` be the iCKB value of all live pool deposits, minus the live iCKB supply, minus the value of all live receipts. Each value is fixed when its cell is created: for a deposit its unused capacity and block, for a receipt its quantity, amount and block. So `P` changes only through transactions. No transaction that keeps the rules lowers it:

- **iCKB Logic runs:** Invariant 1 gives `input xUDT + redeemed receipts = output xUDT + spent deposits`. Invariant 2 makes the receipts created equal in value to the deposits created, because both sit in the same block and a receipt for `q` deposits of amount `A` is worth `q` times the value of one. Invariants 1 and 2 together keep `P` the same.
- **iCKB Logic doesn't run:** no pool deposit is spent (its lock would run). No receipt is created or spent (its type would run). Outside owner mode, xUDT only keeps or burns supply. Owner mode needs iCKB Logic ([xUDT Owner Mode](#xudt-owner-mode)). Deposits created here have no receipt. So `P` stays the same or grows.

`P` starts at zero. So whenever every transaction keeps the per-transaction rules (exactly what the checks above test), `supply + receipts <= deposits` holds on every chain state. A chain reorganization can't break this either (scenario 6E, [whitepaper#15](https://github.com/ickb/whitepaper/issues/15)). A transaction can only enter the chain if every block in its `header_deps` is already in the chain ([RFC 0022](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#header-deps)). So a reorged-out deposit leaves neither a live cell nor a usable header behind.

The live figures match. At mainnet block 20,894,738, live pool deposits are worth exactly the live iCKB supply plus the value of the unredeemed receipts, to the last of the 8 iCKB decimals:

- iCKB supply: 2,199,999.91970001 iCKB over 40 xUDT cells
- unredeemed receipts: 199,999.99299601 iCKB over 2 cells
- pool deposits: 2,399,999.91269602 iCKB over 24 cells

On testnet at block 22,933,531 the deposit side is higher by exactly 10 iCKB. Receiptless deposits and burned iCKB can only push the deposit side higher. So `supply + receipts <= deposits` is the identity worth monitoring.

## Liveness

The invariants say what an accepted transaction can't do. [The liveness tests](scripts/tests/src/tests/fuzz/liveness.rs) check the converse: from each extreme state a user can reach, the next normal transaction of the owner still verifies within one mainnet block (3.5 billion cycles, 597,000 bytes) and keeps the invariants.

- **Largest withdrawal and claim:** 32 deposits of 1,000,000 CKB withdrawn in one transaction, which fills the 64 outputs a transaction with a DAO output may have. Then all 32 are claimed in one transaction. The withdrawal takes 1,743,499 cycles and the claim 1,691,191, about 0.05% of a block each. The same withdrawal also verifies with its deposits at input positions 32 to 63, in blocks past 10,000,000.
- **Long header dep lists:** more than 300 header deps block neither a withdrawal nor a claim. The iCKB scripts find the header of each input through the input itself, never through a header dep index.
- **Receipts and deposits at the edges:** a receipt for 63 deposits of 1,000,000 CKB (the most one deposit transaction can create), plus deposits at both bounds and one shannon around the soft cap. They redeem and withdraw at accumulated rates of genesis, one above it, double it and `u64::MAX`. At `u64::MAX` the soft cap lies above the maximum deposit, so only the two bounds apply.
- **Orders at their end:** fulfilled orders in either direction and empty dust orders melt with their master. A remainder below the minimum match can still be filled in full in either direction, because the minimum binds only partial matches.
- **Wide transactions:** [the reach tests](scripts/tests/src/tests/fuzz/reach.rs) run 150 orders minted, filled and melted at once, 100 withdrawal requests claimed at once and 31 deposits of 31 different amounts. Each also has a case with one wrong entry among the many, which is still refused.

Separately, the type-role scan regression verifies a phase-2 conversion with 512 unrelated outputs within the 10,000,000-cycle ceiling of the suite. Lock-role transactions also execute NervosDAO, whose 64-output limit caps their outputs.

Each script has 516 KB of heap and stops when it runs out. [The reach tests](scripts/tests/src/tests/fuzz/reach.rs) pin where that happens. Each limit bounds one transaction, never a cell: the same cells still move in smaller transactions.

- **Orders per transaction:** how many orders `Limit Order` handles at once depends on the order in which it meets their masters. Sorted masters are the tightest case measured: 382 orders fit, whether minted, filled or melted. One more stops the script. Scattered masters fit more. At 382 the transaction stays under 34 million cycles and 165,000 bytes.
- **Claims per transaction:** 500 claims in one transaction take 39 million cycles, within the 70 million a node accepts into its pool by default. 1,000 claims already take 86 million. `Owned Owner` itself holds 3,063 pairs before its heap runs out.
- **Deposits and receipts:** `iCKB Logic` keeps a small map entry per distinct amount. About a block full of receipts, 3,870 of distinct amounts, still fits its heap.
- **Long output locks:** every script reads each output lock into one heap block. A lock script of up to 256 KiB is read. A longer one stops whichever iCKB script runs. No wallet lock comes near that size.

Two documented limits delay transactions without costing funds:

- **(5A) Witness malleability:** see [whitepaper#22](https://github.com/ickb/whitepaper/issues/22) (credit: @XuJiandong). The scripts use the unsigned script-as-lock plus script-as-type pattern. None of them reads witnesses, so all three witness fields in the `iCKB Logic` lock group are malleable. `NervosDAO` does read `input_type` to locate the deposit header ([`dao.c:63-98`](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L63-L98)). A tampered index then [fails the block-number comparison](https://github.com/nervosnetwork/ckb-system-scripts/blob/814eb82c44f560dbdad2be97eb85464062920237/c/dao.c#L190-L191), as the [iCKB](scripts/tests/src/tests/ickb_logic/dao_phase2_batching.rs) and [Owned Owner](scripts/tests/src/tests/owned_owner/melt_pairing_and_witness.rs) malformed witness tests show. That can fail a transaction, not move funds. The [whitepaper](https://github.com/ickb/whitepaper/blob/master/README.md#unsigned-lock-witnesses-malleability) states the consequence: "if a script in a transaction needs to store data in the witness and this data can be tampered with without making the transaction invalid, then this transaction must not employ the scripts presented in the current whitepaper."
- **(6F) Busywork:** see [whitepaper#8](https://github.com/ickb/whitepaper/issues/8). An attacker can cycle large standard deposits through the pool so the remaining deposits are younger. Per the analysis in that issue, controlling the first available epoch takes about 0.6% of pool capital and the first 3 days about 10%. A user blocked for one epoch loses about 0.0017% interest. The attacker earns nothing, pays fees and locks capital for at least one 180-epoch NervosDAO period per cycle ([RFC 0023](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0023-dao-deposit-withdraw/0023-dao-deposit-withdraw.md)). As the pool grows, the capital needed rises while the impact per unit falls.

## Non-Upgradable Deployment

The reviewed deployment is non-upgradable under the observed script references. Under [RFC 0022](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0022-transaction-structure/0022-transaction-structure.md#code-locating) and [RFC 0032](https://github.com/nervosnetwork/rfcs/blob/4b502ffcb02fc7019e0dd4b5f866b5f09819cfbe/rfcs/0032-ckb-vm-version-selection/0032-ckb-vm-version-selection.md#specification), the deployed scripts in transactions use `hash_type = data1`: referencing scripts locate code in `cell_deps` by cell data hash and run it on CKB VM v1. That reference mode pins validation to the exact deployed binary bytes. The pinning comes from how scripts in transactions refer to the cell, not from the binary cell itself.

By contrast, `hash_type = type` would locate code by type-script hash. That allows a replacement code cell with the same type script and different contents, with the upgrade policy then set by the lock script of that code cell. Separately, the published binary cells are locked with a `secp256k1_blake160` zero lock, an unspendable lock. So no trusted operator key remains as an owner of the binaries. Fixing a bug after deployment would need a migration to new script deployments and a new dep group.

## Conclusion

Under the current deployment assumptions, the four invariants hold on the deployed bytes whenever their owning scripts run. Only the known `Limit Order` confusion attack remains live. The `iCKB Logic` receiptless path exists, but the executed tests stop at a self-funded edge case. The remaining `Owned Owner` and cross-script cases are blocked or confined to integration and deployment boundaries.

## Appendix: Scenario Analysis

The trace of each scenario sits with the invariant it tests, linked to its test where one reproduces it. The others rest on the argument given there.

| ID | Scenario                                              | Result                                      | Trace                                                                     |
| -- | ----------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------- |
| 1A | Direct xUDT minting bypass                            | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 1B | Forge a high-value receipt                            | Blocked                                     | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 1C | Inflate receipt value via AR manipulation             | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 1D | Create a receipt without prior deposits               | Blocked                                     | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 2A | Withdraw deposits without burning iCKB                | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 2B | Bypass iCKB Logic during deposit consumption          | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 2C | Exploit a cell dep for value extraction               | No attack vector                            | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 3A | Oversized deposit fee and discount arbitrage          | No arbitrage                                | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 3B | Cherry-pick the cheapest deposits                     | No mispricing                               | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 3C | Integer rounding exploitation                         | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 4A | Receipt with zero deposit quantity                    | Blocked                                     | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 4B | Receipt amount without a matching deposit             | Blocked                                     | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 4C | Cell with an iCKB Logic lock and a non-DAO type       | Can't be spent                              | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 4D | NervosDAO withdrawal request classified as a deposit  | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 4E | Deposit capacity manipulation                         | Blocked                                     | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 4F | Deposit interest or maturity reset                    | Blocked for the same-index reset            | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 5A | Witness malleability during withdrawal                | Griefing only, documented                   | [Liveness](#liveness)                                                     |
| 5B | Separate lock and type group execution desync         | Blocked                                     | [Script Grouping](#script-grouping)                                       |
| 5C | Non-empty-args iCKB Logic variant                     | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 5D | Orphaned Owned Owner cell                             | Blocked when Owned Owner runs               | [Invariant 4](#invariant-4-withdrawal-claims-stay-one-to-one)             |
| 5E | Limit Order value decrease                            | Blocked                                     | [Invariant 3](#invariant-3-limit-orders-keep-their-value)                 |
| 5F | Limit Order confusion attack (LO-01)                  | Known vulnerability                         | [Invariant 3](#limits-lo-01)                                              |
| 6A | Boundary values                                       | Correct for receipted deposits              | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
| 6B | u128 overflow in the balance equation                 | Blocked                                     | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 6C | Division by zero in deposit_to_ickb                   | Impossible                                  | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 6D | extract_unused_capacity underflow                     | Impossible                                  | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 6E | Chain reorganization between deposit phases           | Blocked by consensus                        | [Whole chain](#from-one-transaction-to-the-whole-chain)                   |
| 6F | Busywork and pool dilution                            | Known limitation                            | [Liveness](#liveness)                                                     |
| 6G | Same-block receipt and deposit consumption            | No profit                                   | [Invariant 1](#invariant-1-no-ickb-from-nothing)                          |
| 6H | Race between receipt creation and deposit withdrawal  | Balanced                                    | [Invariant 2](#invariant-2-receipts-match-deposits)                       |
