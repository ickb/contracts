use super::*;

// Build a plain funding tx that creates an ickb_logic-locked non-DAO output, then try to spend the stranded cell: creation passes because output locks do not run, but the later spend fails because the live cell shape is script misuse.
#[test]
fn lock_only_ickb_logic_non_dao_output_can_be_created() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let helper_type = helper_type_script(&mut context);
    let funding_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(200 * CKB, &ickb_logic, Some(&helper_type)))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("lock-only misuse cell creation bypasses ickb_logic");

    let phantom_out_point = context.create_cell(
        cell(200 * CKB, &ickb_logic, Some(&helper_type)),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(input(phantom_out_point))
        .output(cell(200 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();

    let spend_tx = context.complete_tx(spend_tx);
    fail(&context, &spend_tx, ERROR_SCRIPT_MISUSE);
}

// Build a plain funding tx that creates an ickb_logic lock with non-empty args, then spend that output: creation passes because the lock is only on outputs, but the later spend fails because ickb_logic requires empty args when it finally executes.
#[test]
fn non_empty_args_ickb_logic_lock_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic_non_empty = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    let funding_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(200 * CKB, &ickb_logic_non_empty, None))
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("non-empty-args output lock can be created because output locks do not execute");

    let out_point = context.create_cell(
        cell(200 * CKB, &ickb_logic_non_empty, None),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(input(out_point))
        .output(cell(200 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    fail(&context, &spend_tx, ERROR_NOT_EMPTY_ARGS);
}

// A withdrawal request keeps the deposit's DAO type but carries non-zero data, so it is no deposit;
// locked by ickb_logic it matches no valid shape and the script rejects it rather than treating it as Unknown.
#[test]
fn withdrawal_request_locked_by_ickb_logic_is_script_misuse() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_500 * CKB;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, deposit_amount);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &user_lock, &xudt, u128::from(deposit_amount));

    let withdraw_tx = |request_lock: &Script| {
        TransactionBuilder::default()
            .input(input(deposit_input.clone()))
            .input(input(udt_input.clone()))
            .output(cell(deposit_total_capacity, request_lock, Some(&dao)))
            .output_data(withdrawal_request_data(1554).pack())
            .header_dep(deposit_header.hash())
            .build()
    };

    let control = context.complete_tx(withdraw_tx(&user_lock));
    context
        .verify(&control, MAX_CYCLES)
        .expect("the same request under a user lock is a valid phase1 withdrawal");

    let misuse = context.complete_tx(withdraw_tx(&ickb_logic));
    fail(&context, &misuse, ERROR_SCRIPT_MISUSE);
}
