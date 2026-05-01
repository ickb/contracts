use super::*;

// Continue a real mint lineage into match state but point it at another real master's outpoint; the match path rejects the arbitrary lineage rewrite.
#[test]
fn real_order_match_cannot_rewrite_master_outpoint_to_an_arbitrary_master() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (real_order_out_point, _real_master_out_point) =
        build_real_limit_order_and_master_with_capacity(&mut context, owner_lock.clone(), helper_type.clone(), 1_600 * SHANNONS);
    let (_victim_order_out_point, victim_master_out_point) =
        build_real_limit_order_and_master_with_capacity(&mut context, owner_lock.clone(), helper_type.clone(), 1_500 * SHANNONS);

    let limit_order = limit_order_script(&mut context);
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(real_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &victim_master_out_point, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Continue a real mint-state order into its first match state without spending the master cell; the match path accepts the derived metapoint continuation.
#[test]
fn real_limit_order_can_transition_from_mint_to_match_without_consuming_master() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock, helper_type.clone());

    let limit_order = limit_order_script(&mut context);
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(real_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &real_master_out_point, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("a real minted order should be able to move into a match-state output without consuming its master");
}

// Continue a real mint-state order into match state while changing its ratio info; the match path rejects because the lineage must preserve checked order info.
#[test]
fn real_limit_order_match_rejects_rewriting_order_info() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock, helper_type.clone());

    let limit_order = limit_order_script(&mut context);
    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(real_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &real_master_out_point, (2, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}
