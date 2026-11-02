use super::*;

// Build a phase2 mint whose xUDT output data encodes `u64::MAX + 1`: this exceeds the supported amount ceiling, so verification fails before any mint can escape the xUDT range.
#[test]
fn oversized_output_udt_amount_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let mut spec = mint(&env, env.receipt(1, 1_000 * CKB), u128::from(u64::MAX) + 1, false);
    spec.inputs.insert(0, In::new(env.plain(1_000 * CKB)));
    spec.fail(&mut context, ERROR_AMOUNT_UNREASONABLY_BIG);
}

// Build a plain xUDT transfer that keeps the amount exactly at `u64::MAX`: this is the accepted numeric boundary, so the tx should pass unchanged.
#[test]
fn xudt_amount_at_u64_max_boundary_is_allowed() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let udt = env.udt(u128::from(u64::MAX));
    let spec = TxSpec { inputs: vec![In::new(udt.clone())], outputs: vec![udt.into()], ..Default::default() };
    spec.verify(&mut context).expect("u64::MAX should remain a valid xUDT amount boundary");
}

// Keep an xUDT output at exactly `u64::MAX` while ickb_logic runs (a phase-1 deposit makes it run
// as the receipt's type): its output ceiling admits the boundary, so the transfer verifies. The
// plain transfer above never runs ickb_logic, so only this case pins `>` against `>=`.
#[test]
fn output_udt_amount_at_u64_max_is_allowed_while_ickb_logic_runs() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let udt = env.udt(u128::from(u64::MAX));
    let deposit_amount = 1_000 * CKB;
    let spec = TxSpec {
        inputs: vec![In::new(udt.clone()), In::new(Cell { lock: env.funding.clone(), ..env.plain(2_000 * CKB) })],
        outputs: vec![udt.into(), env.deposit(&env.ickb, deposit_amount).into(), env.receipt(1, deposit_amount).into()],
        ..Default::default()
    };
    spec.verify(&mut context).expect("u64::MAX should remain a valid xUDT output while ickb_logic runs");
}
