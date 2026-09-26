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
        cell(100 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let limit_order = limit_order_script(&mut context);
    let create_tx = TransactionBuilder::default()
        .input(input(old_order_out_point))
        .input(input(old_master_out_point))
        .input(input(funding_input))
        .outputs(vec![
            cell(occupied_capacity(&owner_lock, &limit_order, 0), &owner_lock, Some(&limit_order)),
            cell(100 * CKB, &filler_lock, None),
            cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new(), order_data_mint(0, -2, (1, 1))].pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
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
        .input(input(new_order))
        .input(input(new_master))
        .input(input(funding_cell(&mut context)))
        .output(cell(1_700 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .build();
    let melt_tx = context.complete_tx(melt_tx);
    context
        .verify(&melt_tx, MAX_CYCLES)
        .expect("the reminted sparse pair should remain a valid melt target");
}
