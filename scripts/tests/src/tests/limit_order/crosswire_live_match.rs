use super::*;

// Continue two indistinguishable real orders into permuted match outputs; each output remains redeemable by the master encoded in its data.
#[test]
fn cloned_live_orders_can_permute_without_stranding_either_master() {
    let mut context = Context::default();
    let (owner1_key, owner1_lock, owner1_secp_data_dep) = secp_lock(&mut context);
    let (owner2_key, owner2_lock, owner2_secp_data_dep) = secp_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let (order1_input, master1_input) =
        build_real_limit_order_and_master(&mut context, owner1_lock.clone(), helper_type.clone());
    let (order2_input, master2_input) =
        build_real_limit_order_and_master(&mut context, owner2_lock.clone(), helper_type.clone());

    let limit_order = limit_order_script(&mut context);
    let matched_order1_data = order_data_match(100 * CKB as u128, &master2_input, (1, 1));
    let matched_order2_data = order_data_match(100 * CKB as u128, &master1_input, (1, 1));
    let crosswire_tx = TransactionBuilder::default()
        .input(input(order1_input))
        .input(input(order2_input))
        .outputs(vec![
            cell(1_400 * CKB, &limit_order, Some(&helper_type)),
            cell(1_400 * CKB, &limit_order, Some(&helper_type)),
        ])
        .outputs_data(vec![matched_order1_data.clone(), matched_order2_data.clone()].pack())
        .build();

    let crosswire_tx = context.complete_tx(crosswire_tx);
    context
        .verify(&crosswire_tx, MAX_CYCLES)
        .expect("two cloned live orders with indistinguishable checked state can be re-emitted as matched orders that permute their master metapoints");

    // Materialize both matched outputs and redeem each by its encoded master.
    let crosswired_order1 = seed_verified_output(&mut context, &crosswire_tx, 0, matched_order1_data);
    let crosswired_order2 = seed_verified_output(&mut context, &crosswire_tx, 1, matched_order2_data);

    let rebound_melt = TransactionBuilder::default()
        .input(input(crosswired_order1.clone()))
        .input(input(master2_input.clone()))
        .input(input(funding_cell(&mut context)))
        .output(cell(1_600 * CKB, &owner2_lock, None))
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(owner2_secp_data_dep)
        .build();
    let rebound_melt = sign_tx_by_input_group(context.complete_tx(rebound_melt), &owner2_key, 1, 1);
    context
        .verify(&rebound_melt, MAX_CYCLES)
        .expect("the alternate master can melt the permuted cloned continuation after the metapoint swap");

    let other_melt = TransactionBuilder::default()
        .input(input(crosswired_order2))
        .input(input(master1_input))
        .input(input(funding_cell(&mut context)))
        .output(cell(1_600 * CKB, &owner1_lock, None))
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(owner1_secp_data_dep)
        .build();
    let other_melt = sign_tx_by_input_group(context.complete_tx(other_melt), &owner1_key, 1, 1);
    context
        .verify(&other_melt, MAX_CYCLES)
        .expect("the other permuted continuation should remain redeemable by its encoded master");
}

// Mix a forged match-shaped input with one real order, keep each metapoint on its own output, and confirm only the real lineage still melts with the real master.
#[test]
fn hybrid_fake_and_real_limit_order_match_keeps_real_master_on_real_metapoint() {
    let mut context = Context::default();
    let (owner_key, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let (real_order_input, real_master_input) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type.clone());

    let limit_order = limit_order_script(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);
    let fake_order_input = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_match(0, &fake_master, (1, 1)),
    );

    let real_lineage_data = order_data_match(100 * CKB as u128, &real_master_input, (1, 1));
    let fake_lineage_data = order_data_match(100 * CKB as u128, &fake_master, (1, 1));
    let hybrid_tx = TransactionBuilder::default()
        .input(input(fake_order_input))
        .input(input(real_order_input))
        .outputs(vec![
            cell(1_400 * CKB, &limit_order, Some(&helper_type)),
            cell(1_400 * CKB, &limit_order, Some(&helper_type)),
        ])
        .outputs_data(vec![real_lineage_data.clone(), fake_lineage_data.clone()].pack())
        .build();

    let hybrid_tx = context.complete_tx(hybrid_tx);
    context
        .verify(&hybrid_tx, MAX_CYCLES)
        .expect("a fake match-shaped input can coexist with a real order in one valid match tx");

    let hybrid_hash = hybrid_tx.hash();
    let real_lineage_output = OutPoint::new(hybrid_hash.clone(), 0);
    let fake_lineage_output = OutPoint::new(hybrid_hash, 1);
    context.create_cell_with_out_point(
        real_lineage_output.clone(),
        hybrid_tx.outputs().get(0).expect("real-lineage output"),
        real_lineage_data,
    );
    context.create_cell_with_out_point(
        fake_lineage_output.clone(),
        hybrid_tx.outputs().get(1).expect("fake-lineage output"),
        fake_lineage_data,
    );

    let real_lineage_melt = TransactionBuilder::default()
        .input(input(real_lineage_output))
        .input(input(real_master_input.clone()))
        .input(input(funding_cell(&mut context)))
        .output(cell(1_600 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep.clone())
        .build();
    let real_lineage_melt = sign_tx_by_input_group(context.complete_tx(real_lineage_melt), &owner_key, 1, 1);
    context
        .verify(&real_lineage_melt, MAX_CYCLES)
        .expect("the real master should still authorize the melt on the real metapoint in the hybrid match");

    let fake_lineage_melt_with_real_master = TransactionBuilder::default()
        .input(input(fake_lineage_output))
        .input(input(real_master_input))
        .output(cell(1_600 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .build();
    let fake_lineage_melt_with_real_master = sign_tx_by_input_group(
        context.complete_tx(fake_lineage_melt_with_real_master),
        &owner_key,
        1,
        1,
    );
    fail(&context, &fake_lineage_melt_with_real_master, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
