use super::*;

// Recipient binding depends on the user lock, not Owned Owner itself.
// With a weak owner lock, phase 1 can redirect the owner cell to a different later claimant even though the DAO request itself is unchanged.
#[test]
fn weak_lock_can_reassign_withdrawal_owner_output() {
    let mut context = Context::default();
    let weak_lock = always_success_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        weak_lock,
        deposit_amount,
        1554,
    );

    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(attacker_lock)
        .type_(Some(owned_owner).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("weak lock should allow reassigning the withdrawal owner output");
}

// Scenario: a withdrawal is signed under a secp lock before any tampering.
// Expectation: the signed transaction verifies, but changing the owner output after signing breaks the signature.
#[test]
fn sighash_lock_binds_withdrawal_owner_output_to_the_signed_transaction() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        owner_lock.clone(),
        deposit_amount,
        1554,
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .header_dep(deposit_header.hash())
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 1, 1);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("signed withdrawal request should verify");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner)
                .type_(Some(dao_script(&mut context)).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(attacker_lock)
                .type_(Some(owned_owner_script(&mut context)).pack())
                .build(),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}

// Scenario: a batch mixes one signed owner and one weak owner.
// Expectation: once the signed input group commits to the batch, both withdrawal outputs are bound and post-signing tampering fails.
#[test]
fn mixed_sighash_and_weak_udts_bind_all_withdrawal_outputs_once_signed() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = always_success_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_200 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total1.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total2.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let strong_udt = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(amount1)),
    );
    let weak_udt = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(weak_lock)
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount2)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(strong_udt).build())
        .input(CellInput::new_builder().previous_output(weak_udt).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1555),
                owner_distance_data(-2),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .witness(Bytes::new().pack())
        .cell_dep(secp_data_dep)
        .header_dep(header1.hash())
        .header_dep(header2.hash())
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 2, 1);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("mixed strong+weak withdrawal batch should verify when signed");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner_script(&mut context))
                .type_(Some(dao_script(&mut context)).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner_script(&mut context))
                .type_(Some(dao_script(&mut context)).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner_script(&mut context)).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(attacker_lock)
                .type_(Some(owned_owner_script(&mut context)).pack())
                .build(),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}

// Scenario: both withdrawal owners are weak locks.
// Expectation: nothing binds the owner outputs, so both claims can be reassigned together during phase 1.
#[test]
fn two_weak_udts_can_reassign_combined_withdrawal_owner_outputs() {
    let mut context = Context::default();
    let weak_lock_1 = named_always_success_lock(&mut context, b"weak1");
    let weak_lock_2 = named_always_success_lock(&mut context, b"weak2");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_200 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total1.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total2.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let weak_udt_1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(weak_lock_1)
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(amount1)),
    );
    let weak_udt_2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(weak_lock_2)
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount2)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(weak_udt_1).build())
        .input(CellInput::new_builder().previous_output(weak_udt_2).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(attacker_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1555),
                owner_distance_data(-2),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(header1.hash())
        .header_dep(header2.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("all-weak withdrawal inputs can reassign both withdrawal owner outputs");
}
