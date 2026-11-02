use super::*;

/// Phase2 inputs for receipts of 1,000 and 1,200 CKB from two blocks at different rates, with
/// the iCKB they are worth and their two headers.
fn two_receipts(env: &Env) -> (Vec<In>, u128, [HeaderView; 2]) {
    let first_amount = 1_000 * CKB;
    let second_amount = 1_200 * CKB;
    let headers = [gen_header(0, GENESIS_AR, 0, 0, 1), gen_header(1, SYNTHETIC_WITHDRAW_AR, 1, 1, 1)];
    let inputs = vec![In::new(env.receipt(1, first_amount)).in_block(&headers[0]), In::new(env.receipt(1, second_amount)).in_block(&headers[1])];
    let worth = u128::from(first_amount)
        + (u128::from(second_amount) * u128::from(GENESIS_AR) / u128::from(SYNTHETIC_WITHDRAW_AR));
    (inputs, worth, headers)
}

// Build a phase2 receipt-to-xUDT conversion without including the receipt's header in `header_deps`: the script needs that header to price the receipt, so verification fails with the missing-item path.
#[test]
fn phase2_conversion_without_receipt_header_dep_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let amount = 1_000 * CKB;
    let spec = TxSpec { header_deps: vec![], ..mint(&env, env.receipt(1, amount), u128::from(amount), true) };
    spec.fail(&mut context, ERROR_ITEM_MISSING);
}

// Build one phase2 mint from two receipts that point at two different headers and supply both headers: multi-header conversion is allowed when every receipt has its own dependency, so verification passes.
#[test]
fn phase2_conversion_with_two_receipts_from_distinct_headers_passes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let (inputs, worth, headers) = two_receipts(&env);
    let spec = TxSpec { inputs, outputs: vec![env.udt(worth).into()], header_deps: headers.to_vec() };
    spec.verify(&mut context).expect("phase2 conversion should support multiple receipts from distinct headers");
}

// Build the same two-receipt phase2 shape but omit one of the two required receipt headers: the transaction cannot price every input receipt, so verification fails.
#[test]
fn phase2_conversion_with_one_missing_receipt_header_dep_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let (inputs, _, [first_header, _]) = two_receipts(&env);
    let spec = TxSpec { inputs, outputs: vec![env.udt(u128::from(2_200 * CKB)).into()], header_deps: vec![first_header] };
    spec.fail(&mut context, ERROR_ITEM_MISSING);
}

// Build a phase2 conversion whose receipt header has accumulated rate zero: this is a malformed pricing header, so the conversion path panics instead of minting from an invalid rate.
#[test]
fn phase2_conversion_with_zero_accumulated_rate_header_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let amount = 1_000 * CKB;
    let malformed_header = gen_header(0, 0, 0, 0, 1);
    let spec = TxSpec {
        inputs: vec![In::new(env.receipt(1, amount)).in_block(&malformed_header), env.funding_input()],
        outputs: vec![env.udt(u128::from(amount)).into()],
        header_deps: vec![malformed_header],
    };
    spec.fail(&mut context, ERROR_SCRIPT_PANIC);
}
