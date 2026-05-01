use super::*;

// Start from a forged match-shaped cell that already names a fake master, then keep continuing it; the match path accepts because each step stays self-consistent.
#[test]
fn fake_match_lineage_can_keep_advancing_without_real_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let initial_order = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(0, &fake_master, (1, 1)),
    );

    let first_match_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(initial_order).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &fake_master, (1, 1)).pack())
        .build();

    let first_match_tx = context.complete_tx(first_match_tx);
    context
        .verify_tx(&first_match_tx, MAX_CYCLES)
        .expect("a forged match-shaped order should survive one valid-looking match transition without any real master");

    let first_match_out_point = OutPoint::new(first_match_tx.hash(), 0);
    context.create_cell_with_out_point(
        first_match_out_point.clone(),
        first_match_tx.outputs().get(0).expect("first forged match output"),
        order_data_match(100 * SHANNONS as u128, &fake_master, (1, 1)),
    );

    let second_match_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(first_match_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_300 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(200 * SHANNONS as u128, &fake_master, (1, 1)).pack())
        .build();

    let second_match_tx = context.complete_tx(second_match_tx);
    context
        .verify_tx(&second_match_tx, MAX_CYCLES)
        .expect("the fake match-shaped lineage should stay reusable across multiple later match transitions without any real master");
}

// Forge a match-shaped order that points at a real master, melt through that real master lock, then show the legitimate order is stranded because its master was already consumed.
#[test]
fn fake_match_order_can_strand_real_order() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type.clone());

    let limit_order = limit_order_script(&mut context);
    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order)
            .type_(Some(helper_type).pack())
            .build(),
        order_data_match(0, &real_master_out_point, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(real_master_out_point.clone())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * SHANNONS).pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("a fake match-shaped order should melt successfully against the referenced real master");

    let stranded_order_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(real_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let stranded_order_tx = context.complete_tx(stranded_order_tx);
    let err = context.verify_tx(&stranded_order_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
