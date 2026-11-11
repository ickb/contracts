# iCKB Contracts

NervosDAO deposits, liquid at last!! The NervosDAO protects CKB from inflation, but each deposit stays locked in cycles of about 30 days. iCKB makes them liquid: deposit CKB, get iCKB, then trade it or hold it while it keeps the NervosDAO protection. When it's time to leave, burn iCKB to withdraw from any mature deposit in the shared pool.

Try it now at [ickb.org](https://ickb.org), live on mainnet / testnet!! The [whitepaper](https://github.com/ickb/whitepaper/blob/master/README.md) tells the whole story.

## The Scripts

Three Nervos L1 scripts make it all work:

- **iCKB Logic:** mints / burns iCKB against the pool deposits, so no iCKB exists without the deposits behind it.
- **Owned Owner:** pairs each withdrawal request with its owner, so users can claim their NervosDAO withdrawals.
- **Limit Order:** lets anyone swap CKB with tokens like iCKB through orders, without dealing with NervosDAO rules.

## Security

No admin keys, no upgrades: transactions point at the exact code of these scripts. Their code cells sit under a lock nobody can spend, so nobody can change them, not even their author. That's why they got checked from three sides:

- **Internal review:** people with deep experience in Nervos L1 went through them first, catching most of the issues.
- **External audit:** [Scalebit](https://scalebit.xyz/reports/20240911-ICKB-Final-Audit-Report.pdf) audited them in 2024 (a [local copy](./20240911-ICKB-Final-Audit-Report.pdf) lives here too).
- **Executable review:** [the security review](./ICKB-Audit-Report.md) tests the exact binaries on chain. Fuzzing, exhaustive search, a replay of every iCKB transaction on mainnet / testnet up to the blocks the review names: not a single accepted transaction broke a protocol invariant!! A trace of every machine instruction, plus a reading of what it left, found no untested part that an input can reach and that could change a verdict.

One known issue stays live: anyone can create fake limit orders, which can trick an owner into melting the wrong one. [The review](./ICKB-Audit-Report.md#limits-lo-01) explains it, along with how the iCKB stack picks the real order among them.

Found something else that looks wrong? Please report it in private, as [SECURITY.md](./SECURITY.md) explains.

## Build / Test

[`scripts/README.md`](./scripts/README.md) shows how to rebuild the deployed binaries byte for byte. [`scripts/tests/README.md`](./scripts/tests/README.md) shows how to run the tests. [`scripts/deployment`](./scripts/deployment) holds the deployment of each network.

Building on iCKB? The [iCKB stack](https://github.com/ickb/stack) has the SDK, the liquidity bot and the app behind ickb.org.

## Licensing

This source code, crafted with care by [Phroi](https://phroi.com/), is freely available on [GitHub](https://github.com/ickb/contracts) and it's released under the [MIT License](./LICENSE).
