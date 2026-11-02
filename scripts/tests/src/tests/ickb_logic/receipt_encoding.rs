use super::*;

/// A receipt cell holding `data`, at its occupied capacity.
fn receipt_holding(env: &Env, data: Bytes) -> Cell {
    (Cell { data: data.to_vec(), ..env.receipt(1, 0) }).sized()
}

/// One 1,000 CKB deposit plus a receipt holding `receipt`, funded by a plain input.
fn deposit_with_receipt_data(context: &mut Context, receipt: Bytes) -> TxSpec {
    let env = Env::new(context);
    TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.deposit(&env.ickb, 1_000 * CKB).into(), receipt_holding(&env, receipt).into()],
        ..Default::default()
    }
}

// Build a deposit-creation tx whose receipt output encodes a valid 12-byte receipt plus trailing bytes: creation accounting should ignore the extra suffix, so the deposit and receipt pair still passes.
#[test]
fn receipt_trailing_bytes_do_not_change_creation_accounting() {
    let mut context = Context::default();
    let spec = deposit_with_receipt_data(&mut context, receipt_data_with_trailing_bytes(1, 1_000 * CKB, &[0xaa, 0xbb, 0xcc]));
    spec.verify(&mut context).expect("receipt trailing bytes should not affect accounting");
}

// Build a phase2 mint from a receipt input that has valid receipt fields plus trailing bytes: phase2 parsing should use the canonical prefix only, so conversion still passes.
#[test]
fn receipt_trailing_bytes_do_not_change_phase2_conversion() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let receipt = receipt_holding(&env, receipt_data_with_trailing_bytes(1, 1_000 * CKB, &[0xaa, 0xbb, 0xcc]));
    let spec = mint(&env, receipt, u128::from(1_000 * CKB), true);
    spec.verify(&mut context).expect("receipt trailing bytes should not affect phase2 conversion");
}

// Build a deposit-creation tx whose receipt output truncates the encoded quantity+amount payload to 9 bytes: the receipt shape is incomplete, so creation fails on encoding.
#[test]
fn truncated_receipt_output_with_small_amount_is_rejected() {
    let mut context = Context::default();
    let spec = deposit_with_receipt_data(&mut context, truncated_bytes(receipt_data(1, 1_000 * CKB), 9));
    spec.fail(&mut context, ERROR_ENCODING);
}

// Build a phase2 mint from a live receipt input whose stored receipt payload is truncated to 9 bytes: conversion cannot decode the amount bucket, so verification fails on encoding.
#[test]
fn truncated_receipt_input_with_small_amount_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let receipt = receipt_holding(&env, truncated_bytes(receipt_data(1, 1_000 * CKB), 9));
    let spec = mint(&env, receipt, u128::from(1_000 * CKB), true);
    spec.fail(&mut context, ERROR_ENCODING);
}
