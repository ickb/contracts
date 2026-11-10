# Security

The iCKB scripts are deployed on CKB mainnet and can't be changed. A defect in them can't be patched, so a public report would expose funds with no fix to follow.

## Reporting

Report a suspected vulnerability in private, through [GitHub private vulnerability reporting](https://github.com/ickb/contracts/security/advisories/new). Please don't open a public issue or pull request for it.

A report is easiest to act on with:

- the transaction, or a test against the binaries in `scripts/build/release`, that shows the behavior
- what the script does with it
- which rule it breaks: one of the four invariants of the [security review](ICKB-Audit-Report.md), or a state its owner can no longer spend

## Known Limits

The [security review](ICKB-Audit-Report.md) documents the known limits. Among them: the `Limit Order` confusion attack (LO-01), receiptless deposits and user locks that don't sign the whole transaction. These need no private report.
