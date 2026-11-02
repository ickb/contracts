use super::*;

/// One 1,000 CKB deposit plus a receipt claiming `quantity` deposits of `amount`, funded by a plain input.
fn deposit_with_receipt(context: &mut Context, quantity: u32, amount: u64) -> TxSpec {
    let env = Env::new(context);
    TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.deposit(&env.ickb, 1_000 * CKB).into(), env.receipt(quantity, amount).into()],
        ..Default::default()
    }
}

// Build one deposit plus one receipt whose quantity field is zero: the tx shape includes a receipt cell, but the receipt claims no deposits, so creation fails on the non-empty-receipt invariant.
#[test]
fn zero_quantity_receipt_is_rejected() {
    let mut context = Context::default();
    deposit_with_receipt(&mut context, 0, 1_000 * CKB).fail(&mut context, ERROR_EMPTY_RECEIPT);
}

// Build one deposit plus one receipt that claims quantity 2 for that deposit bucket: the receipt overstates how many equal deposits were created, so matching fails.
#[test]
fn forged_receipt_quantity_without_enough_deposits_is_rejected() {
    let mut context = Context::default();
    deposit_with_receipt(&mut context, 2, 1_000 * CKB).fail(&mut context, ERROR_RECEIPT_MISMATCH);
}

// Build one 1000-CKB deposit plus one receipt that claims a 1001-CKB bucket: receipt matching is keyed by exact deposit amount, so the mismatched bucket is rejected.
#[test]
fn receipt_for_unmatched_deposit_amount_is_rejected() {
    let mut context = Context::default();
    deposit_with_receipt(&mut context, 1, 1_001 * CKB).fail(&mut context, ERROR_RECEIPT_MISMATCH);
}
