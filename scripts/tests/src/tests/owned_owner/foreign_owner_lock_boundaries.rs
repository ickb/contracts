use super::*;

// Creation-time output checks do not block outputs that later become unspendable.
// Phase 1 accepts an owner cell whose lock never executed on creation, but the later claim fails once that foreign lock has to run.
#[test]
fn phase1_accepts_unspendable_foreign_owner_lock() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&poisoned_lock, &owned_owner, 4), &poisoned_lock, Some(&owned_owner)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("owned_owner creation accepts an owner cell whose lock never executed on creation");

    let tx_hash = create_tx.hash();
    let owned_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owner_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(0).expect("owned output"),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(-1),
    );
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let melt_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(123_468_106_670u64, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    fail(&context, &melt_tx, ERROR_NOT_EMPTY_ARGS);
}

// Even an empty-args foreign lock can strand the pair later if the eventual owner lock semantics do not match the claim flow.
#[test]
fn phase1_accepts_limit_order_owner_lock_but_claim_strands() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let benign_foreign_lock = limit_order_script(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&benign_foreign_lock, &owned_owner, 4), &benign_foreign_lock, Some(&owned_owner)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("owned_owner creation accepts an empty-args limit_order owner lock at creation");

    let tx_hash = create_tx.hash();
    let owned_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owner_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(0).expect("owned output"),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(-1),
    );
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let melt_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(123_468_106_670u64, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    fail(&context, &melt_tx, ERROR_ENCODING);
}

// Scenario: phase 1 tries to emit an owner cell whose lock and type are both limit-order driven.
// Expectation: Owned Owner blocks this stronger form immediately instead of letting it reach phase 2.
#[test]
fn limit_order_backed_owner_is_blocked_in_phase1() {
    let mut context = Context::default();
    let (_attacker_privkey, attacker_lock, _secp_data_dep) = secp_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);
    let burn_lock = named_always_success_lock(&mut context, b"owner");

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &burn_lock, &xudt, u128::from(deposit_amount));

    let master_out_point = context.create_cell(
        cell(occupied_capacity(&attacker_lock, &limit_order, 0), &attacker_lock, Some(&limit_order)),
        Bytes::new(),
    );

    let owner_order_data = order_data_match(u128::from(u32::MAX), &master_out_point, (1, 1));
    let create_tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(deposit_capacity(&limit_order, &owned_owner, owner_order_data.len(), 1_500 * CKB), &limit_order, Some(&owned_owner)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_order_data.clone()].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    fail(&context, &create_tx, ERROR_DAO_INCORRECT_CAPACITY);
}
