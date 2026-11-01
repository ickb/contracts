use super::*;

/// A transaction moving a 500 CKB plain input into a 200 CKB output `stranded`, then one spending
/// `stranded` into a plain output.
fn create_then_spend(env: &Env, stranded: Cell) -> (TxSpec, TxSpec) {
    let create = TxSpec { inputs: vec![In::new(env.plain(500 * CKB))], outputs: vec![stranded.clone().into()], ..Default::default() };
    let spend = TxSpec { inputs: vec![In::new(stranded)], outputs: vec![env.plain(200 * CKB).into()], ..Default::default() };
    (create, spend)
}

// Build a plain funding tx that creates an ickb_logic-locked non-DAO output, then try to spend the stranded cell: creation passes because output locks do not run, but the later spend fails because the live cell shape is script misuse.
#[test]
fn lock_only_ickb_logic_non_dao_output_can_be_created() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let stranded = Cell { capacity: 200 * CKB, lock: env.ickb.clone(), type_: Some(env.helper.clone()), data: vec![] };
    let (create, spend) = create_then_spend(&env, stranded);
    create.verify(&mut context).expect("lock-only misuse cell creation bypasses ickb_logic");
    spend.fail(&mut context, ERROR_SCRIPT_MISUSE);
}

// Build a plain funding tx that creates an ickb_logic lock with non-empty args, then spend that output: creation passes because the lock is only on outputs, but the later spend fails because ickb_logic requires empty args when it finally executes.
#[test]
fn non_empty_args_ickb_logic_lock_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let stranded = Cell { lock: env.siblings[0].clone(), ..env.plain(200 * CKB) };
    let (create, spend) = create_then_spend(&env, stranded);
    create.verify(&mut context).expect("non-empty-args output lock can be created because output locks do not execute");
    spend.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// A withdrawal request keeps the deposit's DAO type but carries non-zero data, so it is no deposit;
// locked by ickb_logic it matches no valid shape and the script rejects it rather than treating it as Unknown.
#[test]
fn withdrawal_request_locked_by_ickb_logic_is_script_misuse() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like ickb_logic's, so the control and misuse requests differ only in lock.
    let user_lock = always_success_lock(&mut context);

    let deposit_amount = 1_500 * CKB;
    let deposit = env.deposit(&env.ickb, deposit_amount);
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let inputs = vec![at(deposit.clone(), &deposit_header), In::new(env.udt(u128::from(deposit_amount)))];

    let withdraw = |request_lock: &Script| TxSpec {
        inputs: inputs.clone(),
        outputs: vec![request(&deposit, request_lock, 1554).into()],
        header_deps: vec![deposit_header.clone()],
    };

    withdraw(&user_lock)
        .verify(&mut context)
        .expect("the same request under a user lock is a valid phase1 withdrawal");
    withdraw(&env.ickb).fail(&mut context, ERROR_SCRIPT_MISUSE);
}
