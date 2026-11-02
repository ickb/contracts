use super::*;

// Withdrawal data-shape and header-dependency checks.

/// The canonical withdrawal burning an xUDT cell under `user_lock` that holds `udt_data`, at its occupied capacity.
fn withdrawal_with_udt_data(env: &Env, user_lock: &Script, udt_data: Vec<u8>) -> TxSpec {
    let mut spec = withdrawal(env, user_lock);
    let udt = &mut spec.inputs[1].cell;
    udt.data = udt_data;
    udt.capacity = udt.occupied();
    spec
}

/// Phase 1 withdrawing deposits of 1,000 and 1,200 CKB with one xUDT under `user_lock`.
fn two_deposit_withdrawal(env: &Env, user_lock: &Script) -> TxSpec {
    let mut spec = withdrawals(env, &[1_000 * CKB, 1_200 * CKB]);
    spec.inputs.push(In::new((env).udt(u128::from(2_200 * CKB)).under(user_lock)));
    spec.outputs.push((env).owner(-2).under(user_lock).into());
    spec.outputs.push((env).owner(-2).under(user_lock).into());
    spec
}

// Scenario: the xUDT amount input contains the correct 16-byte amount plus trailing bytes.
// Expectation: Owned Owner reads the prefix amount and accepts the withdrawal.
#[test]
fn withdrawal_accepts_xudt_input_with_trailing_bytes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let mut udt_input_data = udt_data(u128::from(1_000 * CKB)).to_vec();
    udt_input_data.push(0xaa);
    withdrawal_with_udt_data(&env, &user_lock, udt_input_data)
        .verify(&mut context)
        .expect("withdrawal should accept xudt input with trailing bytes");
}

// Scenario: the xUDT input data is shorter than the required 16-byte amount.
// Expectation: amount decoding fails with an encoding error.
#[test]
fn withdrawal_rejects_short_xudt_input_data() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    withdrawal_with_udt_data(&env, &user_lock, truncated_bytes(udt_data(u128::from(1_000 * CKB)), 8).to_vec())
        .fail(&mut context, ERROR_ENCODING);
}

// Scenario: the xUDT input data is completely empty.
// Expectation: amount decoding fails with the same encoding error as other truncated inputs.
#[test]
fn withdrawal_rejects_zero_length_xudt_input_data() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    withdrawal_with_udt_data(&env, &user_lock, vec![]).fail(&mut context, ERROR_ENCODING);
}

// Scenario: the transaction omits the deposit header dep entirely.
// Expectation: Owned Owner cannot resolve the deposit metadata and rejects the withdrawal.
#[test]
fn withdrawal_without_deposit_header_dep_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let mut spec = withdrawal(&env, &user_lock);
    spec.header_deps.clear();
    spec.fail(&mut context, ERROR_ITEM_MISSING);
}

// Scenario: two DAO deposits from different headers are withdrawn together with matching header deps.
// Expectation: the batch succeeds because each withdrawal request can resolve its own deposit header.
#[test]
fn withdrawal_with_two_deposits_from_distinct_headers_passes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let mut spec = two_deposit_withdrawal(&env, &user_lock);
    spec.inputs.push(env.funding_input());
    spec.verify(&mut context).expect("withdrawal should support multiple deposits from distinct headers");
}

// Scenario: one of two deposit headers is omitted from a multi-deposit batch.
// Expectation: the entire batch is rejected because one withdrawal request is missing its header dependency.
#[test]
fn withdrawal_with_one_missing_deposit_header_dep_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let mut spec = two_deposit_withdrawal(&env, &user_lock);
    spec.inputs.push(env.funding_input());
    spec.header_deps.truncate(1);
    spec.fail(&mut context, ERROR_ITEM_MISSING);
}

// Scenario: the deposit header is present but its accumulated rate is zero.
// Expectation: the contract panics on invalid DAO math instead of accepting malformed metadata.
#[test]
fn withdrawal_with_zero_accumulated_rate_deposit_header_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let malformed_header = gen_header(1554, 0, 35, 1000, 1000);
    let mut spec = withdrawal(&env, &user_lock);
    spec.inputs[0].header = Some(malformed_header.clone());
    spec.header_deps = vec![malformed_header];
    spec.fail(&mut context, ERROR_SCRIPT_PANIC);
}

// Scenario: the withdrawal amount is above the soft-cap boundary.
// Expectation: the naive uncapped burn is rejected, while the discounted soft-capped burn amount succeeds.
#[test]
fn withdrawal_applies_soft_cap_discount_above_boundary() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let amount = 100_001 * CKB;
    let mut spec = withdrawals(&env, &[amount]);
    spec.inputs.push(In::new(env.udt(u128::from(amount)).under(&owner_lock)));
    spec.outputs.push(env.owner(-1).under(&owner_lock).into());
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);

    // The same deposit cell, now with the soft-capped burn.
    spec.inputs[1] = In::new(env.udt(soft_capped_ickb(amount, GENESIS_AR)).under(&owner_lock));
    spec.verify(&mut context)
        .expect("withdrawal should require the soft-capped iCKB amount above the 100k boundary");
}
