use super::*;

// Build phase2 receipt-to-xUDT conversions around a reported rounding edge: overstating the mint by even one shannon fails, while the exact header-normalized amount passes, proving the receipt path uses actual shannon precision rather than a coarser rounded claim.
#[test]
fn reported_rounding_claim_is_blocked_by_actual_shannon_precision() {
    let mut context = Context::default();
    let owner_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let ar = 11_509_953_685_250_771u64;
    let amount_1152 = 1_152u64 * SHANNONS;
    let amount_1151 = 1_151u64 * SHANNONS;
    let exact_ickb_1152 = u128::from(amount_1152) * u128::from(GENESIS_AR) / u128::from(ar);
    let exact_ickb_1151 = u128::from(amount_1151) * u128::from(GENESIS_AR) / u128::from(ar);

    assert!(exact_ickb_1152 > 1_000 * SHANNONS as u128);
    assert!(exact_ickb_1151 < exact_ickb_1152);

    let receipt_1152 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(owner_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount_1152),
    );
    let receipt_1152_header = gen_header(1, ar, 1, 1, 1000);
    link_cell_to_header(&mut context, &receipt_1152, &receipt_1152_header);
    let mint_claimed_1000_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_1152.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(owner_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(1_000 * SHANNONS as u128).pack())
        .header_dep(receipt_1152_header.hash())
        .build();
    let mint_claimed_1000_tx = context.complete_tx(mint_claimed_1000_tx);
    let err = context
        .verify_tx(&mint_claimed_1000_tx, MAX_CYCLES)
        .unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let mint_exact_1152_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_1152).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(owner_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(exact_ickb_1152).pack())
        .header_dep(receipt_1152_header.hash())
        .build();
    let mint_exact_1152_tx = context.complete_tx(mint_exact_1152_tx);
    context
        .verify_tx(&mint_exact_1152_tx, MAX_CYCLES)
        .expect("1152 CKB receipt should mint its exact shannon-precision amount");

    let receipt_1151 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(owner_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount_1151),
    );
    let receipt_1151_header = gen_header(2, ar, 1, 1, 1000);
    link_cell_to_header(&mut context, &receipt_1151, &receipt_1151_header);
    let remint_exact_1152_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_1151).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(owner_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(exact_ickb_1152).pack())
        .header_dep(receipt_1151_header.hash())
        .build();
    let remint_exact_1152_tx = context.complete_tx(remint_exact_1152_tx);
    let err = context.verify_tx(&remint_exact_1152_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build phase1 deposit-to-withdrawal conversions around the same rounding claim: burning a rounded-down iCKB amount fails, but burning the exact header-normalized amount passes, so withdrawal initiation also enforces shannon-precise value matching.
#[test]
fn reported_rounding_withdrawal_claim_is_blocked_by_actual_shannon_precision() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let ar = 11_509_953_685_250_771u64;
    let amount_1152 = 1_152u64 * SHANNONS;
    let amount_1151 = 1_151u64 * SHANNONS;
    let exact_ickb_1152 = u128::from(amount_1152) * u128::from(GENESIS_AR) / u128::from(ar);

    let header = gen_header(1554, ar, 35, 1000, 1000);

    let deposit_1152 = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1152).pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_1152, &header);

    let udt_1000 = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(1_000 * SHANNONS as u128),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_1152).build())
        .input(CellInput::new_builder().previous_output(udt_1000).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1152).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(header.hash())
        .build();
    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let deposit_1152_b = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1152).pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_1152_b, &header);
    let udt_exact = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(exact_ickb_1152),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_1152_b).build())
        .input(CellInput::new_builder().previous_output(udt_exact).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1152).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(header.hash())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("withdrawing 1152 CKB should require the exact shannon-precision iCKB amount");

    let deposit_1151 = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1151).pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_1151, &header);
    let udt_exact_again = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(exact_ickb_1152),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_1151).build())
        .input(CellInput::new_builder().previous_output(udt_exact_again).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount_1151).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(header.hash())
        .build();
    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build full phase1-plus-DAO-phase2 flows for older, newer, and soft-capped deposits: each case should pass and the later normalized claim value must stay within one shannon of the phase1 burn, showing the protocol does not open a profitable precision gap across time or size boundaries.
#[test]
fn withdrawal_burn_matches_later_protocol_value_within_one_shannon_across_header_and_soft_cap_cases() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let withdraw_ar = 12_000_000_000_000_000u64;
    let withdraw_header = gen_header(2_000_610, withdraw_ar, 575, 2_000_000, 1100);

    let mut run_case = |amount: u64, deposit_number: u64, deposit_ar: u64| -> u128 {
        let deposit_header = gen_header(deposit_number, deposit_ar, 35, 1000, 1000);
        let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, amount);
        let deposit_occupied_capacity = deposit_total_capacity - amount;

        let deposit_input = context.create_cell(
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            dao_deposit_data(),
        );
        link_cell_to_header(&mut context, &deposit_input, &deposit_header);

        let burned_ickb = soft_capped_ickb(amount, deposit_ar);
        let udt_input = context.create_cell(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            udt_data(burned_ickb),
        );

        let phase1_tx = TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(deposit_input).build())
            .input(CellInput::new_builder().previous_output(udt_input).build())
            .output(
                CellOutput::new_builder()
                    .capacity(deposit_total_capacity.pack())
                    .lock(user_lock.clone())
                    .type_(Some(dao.clone()).pack())
                    .build(),
            )
            .output_data(withdrawal_request_data(deposit_number).pack())
            .header_dep(deposit_header.hash())
            .build();
        let phase1_tx = context.complete_tx(phase1_tx);
        context
            .verify_tx(&phase1_tx, MAX_CYCLES)
            .expect("exact phase1 burn should verify for the deposit header");

        let withdrawal_out_point = OutPoint::new(phase1_tx.hash(), 0);
        let withdrawal_output = phase1_tx.outputs().get(0).expect("withdrawing output");
        context.create_cell_with_out_point(
            withdrawal_out_point.clone(),
            withdrawal_output.clone(),
            withdrawal_request_data(deposit_number),
        );
        link_cell_to_header(&mut context, &withdrawal_out_point, &withdraw_header);

        let claim_capacity = dao_maximum_withdraw_capacity(
            &withdrawal_output,
            withdrawal_request_data(deposit_number).len(),
            deposit_ar,
            withdraw_ar,
        );
        let witness = header_dep_index_witness(1);
        let claim_tx = TransactionBuilder::default()
            .input(
                CellInput::new_builder()
                    .previous_output(withdrawal_out_point)
                    .since(0x2003e800000002f4u64.pack())
                    .build(),
            )
            .output(
                CellOutput::new_builder()
                    .capacity(claim_capacity.pack())
                    .lock(user_lock.clone())
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
            .expect("phase2 claim should verify for the withdrawal request");

        let later_ickb_value =
            soft_capped_ickb(claim_capacity - deposit_occupied_capacity, withdraw_ar);
        assert!(burned_ickb >= later_ickb_value);
        assert!(
            burned_ickb - later_ickb_value <= 1,
            "phase1 burn {burned_ickb} and later normalized claim value {later_ickb_value} should differ by at most one shannon"
        );

        burned_ickb
    };

    let older_deposit_burn = run_case(1_000 * SHANNONS, 1554, GENESIS_AR as u64);
    let newer_deposit_burn = run_case(1_000 * SHANNONS, 1555, 11_000_000_000_000_000u64);
    let oversized_deposit_burn = run_case(150_000 * SHANNONS, 1556, 11_000_000_000_000_000u64);

    assert!(
        older_deposit_burn > newer_deposit_burn,
        "the older deposit should require more nominal iCKB because it is worth more iCKB"
    );
    assert!(
        oversized_deposit_burn > newer_deposit_burn,
        "the oversized deposit should still cost more iCKB than the smaller newer deposit after the haircut"
    );
}
