use super::*;

// Build a plain funding tx that creates an ickb_logic-locked non-DAO output, then try to spend the stranded cell: creation passes because output locks do not run, but the later spend fails because the live cell shape is script misuse.
#[test]
fn lock_only_ickb_logic_non_dao_output_can_be_created() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let helper_type = helper_type_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((500 * CKB).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((200 * CKB).pack())
                .lock(ickb_logic.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("lock-only misuse cell creation bypasses ickb_logic");

    let phantom_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * CKB).pack())
            .lock(ickb_logic)
            .type_(Some(helper_type).pack())
            .build(),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(phantom_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((200 * CKB).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SCRIPT_MISUSE);
}

// Build a plain funding tx that creates an ickb_logic lock with non-empty args, then spend that output: creation passes because the lock is only on outputs, but the later spend fails because ickb_logic requires empty args when it finally executes.
#[test]
fn non_empty_args_ickb_logic_lock_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic_non_empty = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((500 * CKB).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((200 * CKB).pack())
                .lock(ickb_logic_non_empty.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("non-empty-args output lock can be created because output locks do not execute");

    let out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * CKB).pack())
            .lock(ickb_logic_non_empty)
            .build(),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((200 * CKB).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
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
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &xudt, 16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );

    let withdraw_tx = |request_lock: &Script| {
        TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(deposit_input.clone()).build())
            .input(CellInput::new_builder().previous_output(udt_input.clone()).build())
            .output(
                CellOutput::new_builder()
                    .capacity(deposit_total_capacity.pack())
                    .lock(request_lock.clone())
                    .type_(Some(dao.clone()).pack())
                    .build(),
            )
            .output_data(withdrawal_request_data(1554).pack())
            .header_dep(deposit_header.hash())
            .build()
    };

    let control = context.complete_tx(withdraw_tx(&user_lock));
    context
        .verify(&control, MAX_CYCLES)
        .expect("the same request under a user lock is a valid phase1 withdrawal");

    let misuse = context.complete_tx(withdraw_tx(&ickb_logic));
    let err = context.verify(&misuse, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SCRIPT_MISUSE);
}
