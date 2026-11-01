use super::*;

// Scenario: a canonical phase 1 withdrawal creates one owned cell and one matching owner cell.
// Expectation: the matched pair verifies successfully.
#[test]
fn valid_output_pair_passes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    withdrawal(&env, &owner_lock).verify(&mut context).expect("owned_owner should accept a matched output pair");
}

// Pair formation and orphaned-output validation.

// Scenario: a transaction forges an Owned Owner withdrawal-looking pair without consuming any DAO deposit.
// Expectation: the batch is rejected because there is no real withdrawal to pair against.
#[test]
fn withdrawal_shape_cannot_be_created_without_any_dao_input() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = named_always_success_lock(&mut context, b"no-dao-user");
    let funding_lock = named_always_success_lock(&mut context, b"no-dao-funding");
    let withdrawal_capacity = 123_456_780_000u64;
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, withdrawal_capacity + 200))],
        outputs: vec![owned(&env, withdrawal_capacity, 1554).into(), owner(&env, &user_lock, -1).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_DAO_NEWLY_CREATED_CELL);
}

// Scenario: one withdrawal output is paired with two owner outputs in the same batch.
// Expectation: Owned Owner rejects the ambiguous pairing.
#[test]
fn two_owner_cells_for_one_owned_output_are_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = withdrawal(&env, &owner_lock);
    spec.inputs.push(In::new(plain(&env.funding, 200 * CKB)));
    spec.outputs.push(owner(&env, &owner_lock, -2).into());
    spec.fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// The reverse of the case above: two owned withdrawal requests and a single owner cell.
// Expectation: Owned Owner rejects the request left without an owner.
#[test]
fn two_owned_outputs_for_one_owner_cell_are_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let header = deposit_header(1554);
    let deposit = env.deposit(&env.ickb, 1_000 * CKB);
    TxSpec {
        inputs: vec![
            mined(deposit.clone(), &header),
            mined(deposit.clone(), &header),
            In::new(udt(&env, &owner_lock, 2 * u128::from(1_000 * CKB))),
            In::new(plain(&env.funding, 200 * CKB)),
        ],
        outputs: vec![
            owned(&env, deposit.capacity, 1554).into(),
            owned(&env, deposit.capacity, 1554).into(),
            owner(&env, &owner_lock, -2).into(),
        ],
        header_deps: vec![header],
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// Output-lock creation can admit an orphan withdrawal request, but the later claim still fails once Owned Owner executes.
#[test]
fn orphan_withdrawal_request_can_be_created_but_not_claimed() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = named_always_success_lock(&mut context, b"user");
    let mut create = withdrawal(&env, &user_lock);
    create.outputs.truncate(1);
    let live = accept(&mut context, &create, "phase1 accepts an orphan withdrawal request because the owned_owner output lock never executes");

    let withdraw_header = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![claim(live[0].clone(), &withdraw_header, 0x2003e800000002f4, 1)],
        outputs: vec![plain(&user_lock, 123_468_106_670).into()],
        header_deps: vec![withdraw_header, deposit_header(1554)],
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// A type-script orphan is rejected immediately because Owned Owner sees the full output pairing and finds no matching owned cell.
#[test]
fn orphan_owner_output_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 500 * CKB))],
        outputs: vec![owner(&env, &owner_lock, 1).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}
