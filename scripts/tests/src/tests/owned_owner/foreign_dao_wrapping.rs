use super::*;

// Cross-asset wrapping and later claim reassignment.
// This is a pure DAO withdrawal wrapped by Owned Owner, not an iCKB withdrawal.
#[test]
fn foreign_dao_withdrawal_can_be_wrapped_and_claimed() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = named_always_success_lock(&mut context, b"foreign-dao-user");
    let funding_lock = named_always_success_lock(&mut context, b"foreign-dao-funding");

    let deposit_capacity_value = 123_456_780_000u64;
    let deposit_header = deposit_header(1554);
    // A deposit from before block 10,000,000 is exempt from the node's lock-size rule, so one under an
    // ordinary user lock can be wrapped; since then, only a lock of Owned Owner's size can.
    let deposit = Cell { capacity: deposit_capacity_value, ..env.deposit(&user_lock, 0) };
    let create = TxSpec {
        inputs: vec![mined(deposit, &deposit_header), In::new(plain(&funding_lock, 200 * CKB))],
        outputs: vec![owned(&env, deposit_capacity_value, 1554).into(), owner(&env, &user_lock, -1).into()],
        header_deps: vec![deposit_header.clone()],
    };
    let live = accept(&mut context, &create, "owned_owner should accept a foreign DAO withdrawal request pair without any iCKB burn");

    let withdraw_header = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![claim(live[0].clone(), &withdraw_header, 0x2003e800000002f4, 1), live[1].clone()],
        outputs: vec![plain(&user_lock, claim_capacity(&live[0].cell)).into()],
        header_deps: vec![withdraw_header, deposit_header],
    }
    .verify(&mut context)
    .expect("the foreign DAO withdrawal wrapped in owned_owner should remain spendable in DAO phase2");
}
