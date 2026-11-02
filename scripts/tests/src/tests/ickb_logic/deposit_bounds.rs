use super::*;

/// One deposit of `amount` plus its receipt, funded by a plain input.
fn deposit_with_receipt(context: &mut Context, amount: u64) -> TxSpec {
    let env = Env::new(context);
    TxSpec {
        inputs: vec![In::new(env.plain(1_100_000 * CKB))],
        outputs: vec![env.deposit(&env.ickb, amount).into(), env.receipt(1, amount).into()],
        ..Default::default()
    }
}

// Build one DAO deposit plus one matching receipt with amount 999 CKB: this exercises the lower deposit bound, so creation must fail because the claimed deposit bucket is below the minimum supported size.
#[test]
fn deposit_below_minimum_is_rejected() {
    let mut context = Context::default();
    let spec = deposit_with_receipt(&mut context, 999 * CKB);
    spec.fail(&mut context, ERROR_DEPOSIT_TOO_SMALL);
}

// Build one DAO deposit plus one matching receipt with amount exactly 1000 CKB: this is the minimum accepted bucket, so the creation tx should pass at the lower boundary.
#[test]
fn deposit_at_minimum_is_accepted() {
    let mut context = Context::default();
    let spec = deposit_with_receipt(&mut context, 1_000 * CKB);
    spec.verify(&mut context).expect("the deposit should verify");
}

// Build one DAO deposit plus one matching receipt with amount 1,000,001 CKB: this crosses the maximum deposit bucket, so creation must fail because the receipt claims an oversized deposit.
#[test]
fn deposit_above_maximum_is_rejected() {
    let mut context = Context::default();
    let spec = deposit_with_receipt(&mut context, 1_000_001 * CKB);
    spec.fail(&mut context, ERROR_DEPOSIT_TOO_BIG);
}

// Build one DAO deposit plus one matching receipt with amount exactly 1,000,000 CKB: this is the maximum accepted bucket, so the creation tx should still verify at the upper boundary.
#[test]
fn deposit_at_maximum_is_accepted() {
    let mut context = Context::default();
    let spec = deposit_with_receipt(&mut context, 1_000_000 * CKB);
    spec.verify(&mut context).expect("the deposit should verify");
}
