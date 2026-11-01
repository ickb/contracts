use super::*;

// Scenario: an owner cell uses distance `0`, which cannot point to a distinct partner output.
// Expectation: phase 1 rejects the degenerate self-reference.
#[test]
fn zero_distance_owner_output_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 2_000 * CKB))],
        outputs: vec![owned(&env, 1_000 * CKB, 1554).into(), owner(&env, &owner_lock, 0).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// Scenario: a live owner input uses distance `0` during phase 2.
// Expectation: the melt path rejects the self-reference as a mismatch.
#[test]
fn zero_distance_owner_input_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = melt(&env, 8, owner(&env, &owner_lock, 0));
    spec.outputs.push(plain(&owner_lock, 123_468_106_670).into());
    spec.fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// Scenario: the owned cell uses DAO deposit bytes instead of withdrawal-request bytes.
// Expectation: Owned Owner rejects the cell because it is not wrapping a withdrawal request.
#[test]
fn non_withdrawal_owned_cell_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_shaped = Cell { data: dao_deposit_data().to_vec(), ..owned(&env, 1_000 * CKB, 1554) };
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 2_000 * CKB))],
        outputs: vec![deposit_shaped.into(), owner(&env, &owner_lock, -1).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// A lock-only Owned Owner look-alike can be created with arbitrary type data, but the first spend fails once Owned Owner actually classifies the cell.
#[test]
fn lock_only_owned_owner_non_dao_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let look_alike = Cell { type_: Some(env.helper.clone()), ..plain(&env.owned_owner, 200 * CKB) };
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 500 * CKB))],
        outputs: vec![look_alike.clone().into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("lock-only owned_owner misuse cell can be created because output locks do not execute");

    TxSpec {
        inputs: vec![In::new(look_alike)],
        outputs: vec![plain(&always_success_lock(&mut context), 200 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// The same creation-time gap allows a lock-only DAO-shaped look-alike, but the later spend still fails because it is not a withdrawal request pair.
#[test]
fn lock_only_owned_owner_dao_deposit_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let look_alike = Cell { data: dao_deposit_data().to_vec(), ..owned(&env, 1_000 * CKB, 1554) };
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 2_000 * CKB))],
        outputs: vec![look_alike.clone().into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("DAO deposit with owned_owner only on the output lock can be created");

    TxSpec {
        inputs: vec![In::new(look_alike)],
        outputs: vec![plain(&always_success_lock(&mut context), 1_000 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// Scenario: the same script is used as both lock and type on one cell.
// Expectation: Owned Owner rejects this explicit script misuse immediately.
#[test]
fn cell_using_owned_owner_as_both_lock_and_type_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 500 * CKB))],
        outputs: vec![owner(&env, &env.owned_owner, 0).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_SCRIPT_MISUSE);
}

// Scenario: the owned cell carries withdrawal-request-shaped data (8 non-zero bytes) under a
// non-DAO type.
// Expectation: Owned Owner rejects it on the DAO type check alone; the other look-alike tests
// fail the data check first, so only this case pins the type check.
#[test]
fn request_shaped_owned_cell_without_dao_type_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let request_shaped = Cell { type_: Some(env.helper.clone()), ..owned(&env, 1_000 * CKB, 1554) };
    TxSpec {
        inputs: vec![In::new(plain(&funding_lock, 2_000 * CKB))],
        outputs: vec![request_shaped.into(), owner(&env, &owner_lock, -1).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}
