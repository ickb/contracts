use super::*;

// Melt one real pair and remint a new sparse negative-distance pair in the same transaction; the old master path closes cleanly and the new pair remains meltable.
#[test]
fn can_atomically_melt_and_remint_with_negative_distance_and_filler() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let funding_lock = always_success_lock(&mut context);
    let (old_order_out_point, old_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type.clone());
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(100u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let limit_order = limit_order_script(&mut context);
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(old_order_out_point).build())
        .input(CellInput::new_builder().previous_output(old_master_out_point).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(100u64.pack())
                .lock(filler_lock)
                .build(),
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new(), order_data_mint(0, -2, (1, 1))].pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("limit_order should allow canceling one pair while minting a new sparse negative-distance pair in the same tx");

    let tx_hash = create_tx.hash();
    let new_master = OutPoint::new(tx_hash.clone(), 0);
    let new_order = OutPoint::new(tx_hash, 2);
    context.create_cell_with_out_point(
        new_master.clone(),
        create_tx.outputs().get(0).expect("new master"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        new_order.clone(),
        create_tx.outputs().get(2).expect("new order"),
        order_data_mint(0, -2, (1, 1)),
    );

    let melt_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(new_order).build())
        .input(CellInput::new_builder().previous_output(new_master).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * SHANNONS).pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let melt_tx = context.complete_tx(melt_tx);
    context
        .verify_tx(&melt_tx, MAX_CYCLES)
        .expect("the reminted sparse pair should remain a valid melt target");
}
