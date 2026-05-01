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
            .capacity(500u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(ickb_logic.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("lock-only misuse cell creation bypasses ickb_logic");

    let phantom_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(ickb_logic)
            .type_(Some(helper_type).pack())
            .build(),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(phantom_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
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
            .capacity(500u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(ickb_logic_non_empty.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("non-empty-args output lock can be created because output locks do not execute");

    let out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(ickb_logic_non_empty)
            .build(),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}
