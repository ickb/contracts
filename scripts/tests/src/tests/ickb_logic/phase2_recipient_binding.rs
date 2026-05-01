use super::*;

// Build a phase2 mint from a receipt protected only by an always-success lock and send the xUDT to an attacker lock: no signature binds the output recipient, so reassignment is expected to pass.
#[test]
fn weak_lock_receipt_can_reassign_phase2_mint_recipient() {
    let mut context = Context::default();
    let weak_lock = always_success_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (receipt_out_point, receipt_header) = create_receipt_input(
        &mut context,
        weak_lock,
        &ickb_logic,
        receipt_data(1, deposit_amount),
        0,
        GENESIS_AR,
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(receipt_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(attacker_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("weak lock should allow reassigning the phase2 mint recipient");
}

// Build a signed phase2 mint from a secp-protected receipt, then tamper the xUDT recipient after signing: the original tx passes, but the modified output shape fails because sighash binds the minted recipient to the signed transaction.
#[test]
fn sighash_lock_binds_phase2_mint_outputs_to_the_signed_transaction() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(owner_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, deposit_amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(owner_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .header_dep(receipt_header.clone())
        .build();

    let tx = sign_tx(context.complete_tx(tx), &privkey);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("signed owner transaction should verify");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(attacker_lock)
                .type_(Some(xudt).pack())
                .build(),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}

// Build one phase2 mint that mixes a signed receipt and a weak receipt into one xUDT output, then tamper the recipient after signing the strong input group: the signed tx passes, but the tampered version fails because one strong lock is enough to bind the shared outputs.
#[test]
fn mixed_sighash_and_weak_receipts_bind_all_phase2_outputs_once_signed() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = always_success_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_200 * SHANNONS;
    let receipt1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(owner_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount1),
    );
    let receipt2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(weak_lock)
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount2),
    );
    let header1 = insert_header_for_cell(&mut context, &receipt1, 0, GENESIS_AR);
    let header2 = gen_header(1, SYNTHETIC_WITHDRAW_AR, 1, 1, 1);
    link_cell_to_header(&mut context, &receipt2, &header2);
    let expected = u128::from(amount1)
        + (u128::from(amount2) * u128::from(GENESIS_AR) / u128::from(SYNTHETIC_WITHDRAW_AR));

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt1).build())
        .input(CellInput::new_builder().previous_output(receipt2).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(owner_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(expected).pack())
        .witness(empty_witness().pack())
        .witness(Bytes::new().pack())
        .cell_dep(secp_data_dep)
        .header_dep(header1)
        .header_dep(header2.hash())
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 0, 1);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("mixed strong+weak phase2 tx should verify when signed");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(attacker_lock)
                .type_(Some(xudt).pack())
                .build(),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}

// Build one phase2 mint from two weakly locked receipts and pay the combined xUDT to an attacker lock: with no strong signer in the transaction, nothing binds the recipient, so reassignment is expected to pass.
#[test]
fn two_weak_receipts_can_reassign_combined_phase2_mint_recipient() {
    let mut context = Context::default();
    let weak_lock_1 = named_always_success_lock(&mut context, b"weak1");
    let weak_lock_2 = named_always_success_lock(&mut context, b"weak2");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_200 * SHANNONS;
    let receipt1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(weak_lock_1)
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount1),
    );
    let receipt2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(weak_lock_2)
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount2),
    );
    let header1 = insert_header_for_cell(&mut context, &receipt1, 0, GENESIS_AR);
    let header2 = gen_header(1, SYNTHETIC_WITHDRAW_AR, 1, 1, 1);
    link_cell_to_header(&mut context, &receipt2, &header2);
    let expected = u128::from(amount1)
        + (u128::from(amount2) * u128::from(GENESIS_AR) / u128::from(SYNTHETIC_WITHDRAW_AR));

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt1).build())
        .input(CellInput::new_builder().previous_output(receipt2).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(attacker_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(expected).pack())
        .header_dep(header1)
        .header_dep(header2.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("all-weak phase2 inputs can reassign the combined phase2 mint recipient");
}
