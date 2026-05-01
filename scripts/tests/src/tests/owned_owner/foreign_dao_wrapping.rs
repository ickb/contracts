use super::*;

// Cross-asset wrapping and later claim reassignment.
// This is a pure DAO withdrawal wrapped by Owned Owner, not an iCKB withdrawal.
#[test]
fn foreign_dao_withdrawal_can_be_wrapped_and_claimed() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"foreign-dao-user");
    let funding_lock = named_always_success_lock(&mut context, b"foreign-dao-funding");
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let deposit_capacity_value = 123_456_780_000u64;
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity_value.pack())
            .lock(user_lock.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity_value.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("owned_owner should accept a foreign DAO withdrawal request pair without any iCKB burn");

    let tx_hash = create_tx.hash();
    let owned_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owner_out_point = OutPoint::new(tx_hash, 1);
    let owned_output = create_tx.outputs().get(0).expect("owned output");
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        owned_output.clone(),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let witness = header_dep_index_witness(1);
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(
                    dao_maximum_withdraw_capacity(
                        &owned_output,
                        withdrawal_request_data(1554).len(),
                        GENESIS_AR as u64,
                        SYNTHETIC_WITHDRAW_AR,
                    )
                    .pack(),
                )
                .lock(user_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify_tx(&claim_tx, MAX_CYCLES)
        .expect("the foreign DAO withdrawal wrapped in owned_owner should remain spendable in DAO phase2");
}
