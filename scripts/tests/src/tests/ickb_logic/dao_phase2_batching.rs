use super::*;

/// A DAO phase2 claim of one withdrawal request whose input carries `witness`.
fn claim_with_witness(context: &mut Context, witness: Bytes) -> TxSpec {
    let owner_lock = always_success_lock(context);
    let dao = dao_script(context);
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let withdrawing = Cell {
        capacity: 123_456_780_000,
        lock: owner_lock.clone(),
        type_: Some(dao),
        data: withdrawal_request_data(1554).to_vec(),
    };
    TxSpec {
        inputs: vec![In { header: Some(withdraw_header.clone()), since: 0x2003e800000002f4, witness, ..In::new(withdrawing) }],
        outputs: vec![Cell { capacity: 123_468_106_670, lock: owner_lock, type_: None, data: vec![] }.into()],
        header_deps: vec![withdraw_header, deposit_header],
    }
}

// Build one DAO phase2 input that tries to fan out into 65 plain outputs: this hits the upstream DAO output-count limit, so verification must fail even before any higher-level accounting question matters.
#[test]
fn dao_phase2_with_65_outputs_hits_the_upstream_batch_limit() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let dao = dao_script(&mut context);

    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let deposit_number_data = 1554u64.to_le_bytes();
    let withdrawing_input = context.create_cell(
        cell(1_234_567_800u64 * 65, &owner_lock, Some(&dao)),
        Bytes::from(deposit_number_data.to_vec()),
    );
    link_cell_to_header(&mut context, &withdrawing_input, &withdraw_header);
    context.insert_header(deposit_header.clone());

    // Funds the payout beyond the one claim's maximum withdraw, so only the DAO's 64-output limit fails.
    let funding_lock = always_success_lock(&mut context);
    let funding = context.create_cell(cell(100_000 * CKB, &funding_lock, None), Bytes::new());

    let outputs = vec![
        cell(123_468_105_678u64, &owner_lock, None);
        65
    ];
    let outputs_data = vec![Bytes::new(); 65];
    let witness = header_dep_index_witness(1);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(withdrawing_input)
                .since(0x2003e8022a0002f3u64)
                .build(),
        )
        .input(input(funding))
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .witness(Bytes::new().pack())
        .cell_dep(secp_data_dep)
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 0, 1);
    fail(&context, &tx, ERROR_DAO_TOO_MANY_OUTPUT_CELLS);
}

// Build a helper-generated phase2 batch with 64 withdrawal claims and 64 matching deposit headers: this is the largest accepted batch shape, so verification should pass at the boundary.
#[test]
fn dao_phase2_with_64_outputs_from_64_distinct_headers_passes() {
    let (context, tx) = build_many_header_phase2_batch(64, None);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("64-output DAO phase2 batch with distinct deposit headers should verify");
}

// Build a single-withdrawal DAO phase2 tx but put the header-dep index bytes in `output_type`: the witness is in the wrong slot, so the DAO parser rejects the shape and verification fails.
#[test]
fn dao_phase2_rejects_header_dep_index_witness_in_output_type() {
    let mut context = Context::default();
    let spec = claim_with_witness(&mut context, witness_with_output_type(Bytes::from(1u64.to_le_bytes().to_vec())));
    let tx = spec.build(&mut context);
    // The node reads the same witness slot for the deposit header, so it rejects the misplaced index too.
    assert_script_error(context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "InvalidDaoFormat").unwrap_err(), -11);
}

// Build a single-withdrawal DAO phase2 tx with an `input_type` witness that truncates the 8-byte header-dep index: the index encoding is incomplete, so DAO validation fails.
#[test]
fn dao_phase2_rejects_short_header_dep_index_witness_in_input_type() {
    let mut context = Context::default();
    let spec = claim_with_witness(&mut context, witness_with_input_type(truncated_bytes(Bytes::from(1u64.to_le_bytes().to_vec()), 4)));
    let tx = spec.build(&mut context);
    // The node decodes the same header index, so it rejects the truncated one too.
    assert_script_error(context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "InvalidDaoFormat").unwrap_err(), -11);
}

// Build a 64-item DAO phase2 batch but deliberately point one witness at the wrong deposit header: the batch shape is otherwise valid, so failure isolates the per-input header binding invariant.
#[test]
fn dao_phase2_rejects_misbound_deposit_header_index_in_large_batch() {
    let (context, tx) = build_many_header_phase2_batch(64, Some((37, 1)));
    // The node resolves the deposit header through the same witness index, so it rejects the misbinding too.
    assert_script_error(context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "InvalidOutPoint").unwrap_err(), ERROR_DAO_INVALID_WITHDRAW_BLOCK);
}

// Build one tx that claims two withdrawal cells from two different deposit headers with separate header indices: mixed-header phase2 batching is allowed when each input names its own deposit header, so verification should pass.
#[test]
fn dao_phase2_with_two_distinct_deposit_headers_passes() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let dao = dao_script(&mut context);

    let deposit_header_1 = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let deposit_header_2 = gen_header(1564, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header_1 = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let withdraw_header_2 = gen_header(2_000_621, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let input_1_output = cell(123_456_780_000u64, &owner_lock, Some(&dao));
    let input_1 = context.create_cell(input_1_output.clone(), withdrawal_request_data(1554));
    let input_2_output = cell(123_456_781_000u64, &owner_lock, Some(&dao));
    let input_2 = context.create_cell(input_2_output.clone(), withdrawal_request_data(1564));
    link_cell_to_header(&mut context, &input_1, &withdraw_header_1);
    link_cell_to_header(&mut context, &input_2, &withdraw_header_2);
    context.insert_header(deposit_header_1.clone());
    context.insert_header(deposit_header_2.clone());

    let witness_1 = header_dep_index_witness(2);
    let witness_2 = header_dep_index_witness(3);
    let exact_capacity_1 = dao_maximum_withdraw_capacity(
        &input_1_output,
        withdrawal_request_data(1554).len(),
        SYNTHETIC_DEPOSIT_AR,
        SYNTHETIC_WITHDRAW_AR,
    );
    let exact_capacity_2 = dao_maximum_withdraw_capacity(
        &input_2_output,
        withdrawal_request_data(1564).len(),
        SYNTHETIC_DEPOSIT_AR,
        SYNTHETIC_WITHDRAW_AR,
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(input_1)
                .since(0x2003e800000002f4u64)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(input_2)
                .since(0x2003e802340002f3u64)
                .build(),
        )
        .output(cell(exact_capacity_1, &owner_lock, None))
        .output(cell(exact_capacity_2, &owner_lock, None))
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .header_dep(withdraw_header_1.hash())
        .header_dep(withdraw_header_2.hash())
        .header_dep(deposit_header_1.hash())
        .header_dep(deposit_header_2.hash())
        .witness(witness_1.pack())
        .witness(witness_2.pack())
        .cell_dep(secp_data_dep)
        .build();

    let tx = sign_tx(context.complete_tx(tx), &privkey);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("two-header DAO phase2 batch should verify");
}
