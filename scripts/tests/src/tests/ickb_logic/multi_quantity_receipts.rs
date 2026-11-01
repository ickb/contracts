use super::*;

// Build a creation tx with two equal DAO deposits and one receipt whose quantity field is 2: this checks that receipt accounting can collapse repeated same-amount buckets, so the tx should pass.
#[test]
fn repeated_deposit_bucket_can_be_matched_by_one_multi_quantity_receipt() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let deposit = env.deposit(&env.ickb, 1_000 * CKB);
    let spec = TxSpec {
        inputs: vec![In::new(env.plain(3_000 * CKB))],
        outputs: vec![deposit.clone().into(), deposit.into(), env.receipt(2, 1_000 * CKB).into()],
        ..Default::default()
    };
    spec.verify(&mut context).expect("one receipt with quantity 2 should match two equal-sized deposits");
}

// Build a phase2 conversion that spends one quantity-2 receipt into one xUDT output: the receipt should mint the combined value of both deposits, so verification passes.
#[test]
fn multi_quantity_receipt_can_be_converted_in_phase2() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let spec = mint(&env, env.receipt(2, 1_000 * CKB), u128::from(2 * 1_000 * CKB), true);
    spec.verify(&mut context).expect("a valid receipt with quantity 2 should mint the combined iCKB amount in phase2");
}
