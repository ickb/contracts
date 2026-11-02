use super::*;

/// A valid phase2 mint of a 1,000 CKB receipt with a plain input of `extra` capacity after it and
/// `sibling` as a second output; `funded` adds the wallet's funding input last.
fn mint_beside(context: &mut Context, extra: u64, funded: bool, sibling: impl FnOnce(&mut Context, &Env) -> Cell) -> TxSpec {
    let env = Env::new(context);
    let amount = 1_000 * CKB;
    let mut spec = mint(&env, env.receipt(1, amount), u128::from(amount), funded);
    spec.inputs.insert(1, In::new(env.plain(extra)));
    spec.outputs.push(sibling(context, &env).into());
    spec
}

/// A cell under `lock` with no type, holding `data`.
fn locked_by(lock: &Script, capacity: u64, data: Bytes) -> Cell {
    Cell { capacity, lock: lock.clone(), type_: None, data: data.to_vec() }
}

// Build a valid receipt-to-xUDT phase2 mint but add an unrelated output lock that reuses ickb_logic code with non-empty args: sibling classification still executes that shape, so the whole tx fails on the empty-args invariant.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, 500 * CKB, false, |_, env| locked_by(&env.siblings[0], 200 * CKB, Bytes::new()));
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Build a valid phase2 mint but add another output whose lock is the iCKB xUDT script itself: this sibling looks like script misuse rather than a normal foreign output, so verification fails.
#[test]
fn ickb_xudt_shaped_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, 500 * CKB, false, |_, env| locked_by(&env.xudt, capacity_for_data(16), udt_data(0)));
    spec.fail(&mut context, ERROR_SCRIPT_MISUSE);
}

// Build a valid phase2 mint but add another output whose lock is the DAO script with deposit-shaped data: that sibling is an impossible lock shape the classifier rejects, so the transaction fails as misuse.
#[test]
fn dao_deposit_shaped_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, 500 * CKB, false, |_, env| locked_by(&env.dao, capacity_for_data(8), dao_deposit_data()));
    spec.fail(&mut context, ERROR_SCRIPT_MISUSE);
}

// Build a valid phase2 mint plus a second output whose lock uses the same xUDT code under different args: the sibling is foreign to iCKB accounting, so the real mint should still pass.
#[test]
fn foreign_xudt_output_lock_is_ignored() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, capacity_for_data(16), true, |context, _| {
        let foreign_owner = named_always_success_lock(context, b"foreign-owner");
        locked_by(&xudt_script(context, &foreign_owner), capacity_for_data(16), udt_data(0))
    });
    spec.verify(&mut context).expect("same xudt code with different args should stay outside iCKB classification");
}

// Build a valid phase2 mint plus a second typed output that uses the same xUDT code under different args: the accounting boundary is script hash plus args, so this foreign xUDT type should be ignored and the tx passes.
#[test]
fn foreign_xudt_type_output_is_ignored() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, capacity_for_data(16), true, |context, env| {
        let foreign_owner = named_always_success_lock(context, b"foreign-owner");
        Cell { type_: Some(xudt_script(context, &foreign_owner)), ..env.udt(0) }
    });
    spec.verify(&mut context).expect("same xudt code with different args should stay outside iCKB UDT accounting");
}

// Build a valid phase2 mint plus a DAO-locked output that carries withdrawal-request data instead of deposit data: this sibling should stay outside deposit classification, so the mint still passes.
#[test]
fn withdrawal_request_shaped_dao_output_lock_is_ignored() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, capacity_for_data(8), true, |_, env| locked_by(&env.dao, capacity_for_data(8), withdrawal_request_data(1554)));
    spec.verify(&mut context).expect("non-zero dao-shaped lock outputs should remain outside deposit classification");
}

// Build a valid phase2 mint plus an output that closely resembles an iCKB deposit but uses non-empty ickb_logic args: the plausible deposit shape still violates the args invariant, so verification fails.
#[test]
fn deposit_shaped_non_empty_args_output_poisons_phase2() {
    let mut context = Context::default();
    let spec = mint_beside(&mut context, 2_000 * CKB, false, |_, env| env.deposit(&env.siblings[0], 1_000 * CKB));
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Build a valid phase2 mint but include a second receipt input whose type script reuses ickb_logic code with non-empty args: sibling input classification sees an invalid receipt-shaped peer, so the whole mint fails on the args check.
#[test]
fn non_empty_args_receipt_sibling_poisons_phase2() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let amount = 1_000 * CKB;
    let mut spec = mint(&env, env.receipt(1, amount), u128::from(amount), false);
    let poisoned_receipt = (Cell { type_: Some(env.siblings[0].clone()), ..env.receipt(1, amount) }).sized();
    spec.inputs.push(In::new(poisoned_receipt).in_block(&spec.header_deps[0]));
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}
