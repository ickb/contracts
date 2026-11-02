use super::*;

// Build one tx that both creates a fresh deposit+receipt pair and converts an old receipt to xUDT, but overstates the minted xUDT by one shannon: the mixed flow must still conserve value, so verification fails.
#[test]
fn mixed_flow_cannot_overmint_by_combining_new_deposit_with_phase2_receipt() {
    let mut context = Context::default();
    let env = Env::new(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_header = gen_header(0, GENESIS_AR, 0, 0, 1);
    let spec = TxSpec {
        inputs: vec![In::new(env.receipt(1, deposit_amount)).in_block(&receipt_header), In::new(env.plain(2_000 * CKB))],
        outputs: vec![
            env.deposit(&env.ickb, deposit_amount).into(),
            env.receipt(1, deposit_amount).into(),
            env.udt(u128::from(deposit_amount) + 1).into(),
        ],
        header_deps: vec![receipt_header],
    };
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Build one tx that simultaneously creates a new deposit, starts a withdrawal from another deposit, reissues a receipt, and remints xUDT by one extra shannon: even with phase1 and phase2 combined, the cross-flow accounting invariant should reject the overmint.
#[test]
fn mixed_flow_cannot_overmint_when_deposit_phase1_phase2_and_withdrawal_share_one_tx() {
    let mut context = Context::default();
    let env = Env::new(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_header = gen_header(0, GENESIS_AR, 0, 0, 1);
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let deposit = env.deposit(&env.ickb, deposit_amount);

    let spec = TxSpec {
        inputs: vec![
            In::new(env.receipt(1, deposit_amount)).in_block(&receipt_header),
            In::new(deposit.clone()).in_block(&deposit_header),
            In::new(env.udt(u128::from(deposit_amount))),
            In::new(env.plain(2_000 * CKB)),
        ],
        outputs: vec![
            deposit.clone().into(),
            request(&deposit, &env.owned_owner, 1554).into(),
            env.owner(-1).into(),
            env.receipt(1, deposit_amount).into(),
            env.udt(u128::from(deposit_amount) + 1).into(),
        ],
        header_deps: vec![receipt_header, deposit_header],
    };
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Build one live transition that consumes a receipt, a DAO deposit, and a real limit order, then emits a new limit order plus a withdrawal pair: all three script families should compose cleanly when each sibling shape and amount is valid, so verification passes.
#[test]
fn all_three_scripts_can_compose_in_one_live_state_transition() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let helper_type = helper_type_script(&mut context);
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type.clone());

    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);

    let receipt_input = create_receipt(&mut context, &owner_lock, &ickb_logic, 1, deposit_amount);
    link_cell_to_header(&mut context, &receipt_input, &deposit_header);

    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let tx = TransactionBuilder::default()
        .input(input(receipt_input))
        .input(input(deposit_input))
        .input(input(real_order_out_point))
        .outputs(vec![
            cell(1_400 * CKB, &limit_order, Some(&helper_type)),
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
        ])
        .outputs_data(
            vec![
                order_data_match(100 * CKB as u128, &real_master_out_point, (1, 1)),
                withdrawal_request_data(1554),
                owner_distance_data(-1),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("a live receipt, live deposit, and live limit order should compose in one valid transaction");
}

// Scenario 4F: withdrawing a deposit and re-depositing its value in one transaction must leave the
// withdrawal request at the deposit's index. Putting the fresh deposit there instead would restart
// the deposit's age in place; NervosDAO phase 1 rejects that layout.
#[test]
fn fresh_deposit_cannot_replace_a_withdrawn_deposit_at_its_index() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let amount = 1_500 * CKB;
    let deposit = env.deposit(&env.ickb, amount);
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let inputs = vec![
        In::new(deposit.clone()).in_block(&deposit_header),
        In::new(env.udt(u128::from(amount))),
        In::new(env.plain(2 * deposit.capacity)),
    ];

    let request = request(&deposit, &user_lock, 1554);
    let receipt = env.receipt(1, amount);
    let tx = |outputs: [&Cell; 3]| TxSpec {
        inputs: inputs.clone(),
        outputs: outputs.into_iter().map(|c| c.clone().into()).collect(),
        header_deps: vec![deposit_header.clone()],
    };

    tx([&request, &deposit, &receipt])
        .verify(&mut context)
        .expect("withdrawing with the request at the deposit's index and re-depositing elsewhere verifies");

    // Same cells with the two DAO outputs swapped: the fresh deposit takes the withdrawn deposit's index.
    tx([&deposit, &request, &receipt]).fail(&mut context, ERROR_DAO_INVALID_WITHDRAWING_CELL);
}

// Scenario 6G: a deposit and its receipt created in the same block read the same accumulated rate,
// so a later transaction spending both (receipt to phase 2, deposit to withdrawal) mints nothing.
#[test]
fn same_block_receipt_and_deposit_mint_nothing_when_spent_together() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let amount = 1_500 * CKB;
    let deposit = env.deposit(&env.ickb, amount);
    let creation_header = gen_header(1554, SYNTHETIC_WITHDRAW_AR, 35, 1000, 1000);
    let inputs = vec![In::new(deposit.clone()).in_block(&creation_header), In::new(env.receipt(1, amount)).in_block(&creation_header)];

    for minted in [0u128, 1] {
        let mut spec = TxSpec {
            inputs: inputs.clone(),
            outputs: vec![request(&deposit, &user_lock, 1554).into()],
            header_deps: vec![creation_header.clone()],
        };
        if minted > 0 {
            spec.outputs.push(env.udt(minted).into());
        }
        let result = spec.verify(&mut context);
        if minted == 0 {
            result.expect("the receipt's value exactly pays for the same-rate deposit");
        } else {
            assert_script_error(result.unwrap_err(), ERROR_AMOUNT_MISMATCH);
        }
    }
}

// A cell can be a limit order and an iCKB receipt at once: lock limit_order, type ickb_logic. Each
// script reads its own prefix of the same data, a receipt from the first 12 bytes and the order's
// UDT amount from the first 16. Created beside its deposit and master, then matched, both rules
// hold: raising the UDT field by one raises the receipt's quantity by one, so the match must also
// create that many more deposits, and the redeemed receipt mints exactly its value.
#[test]
fn receipt_typed_order_keeps_both_rules_through_a_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let amount = 1_000 * CKB;
    // Quantity in bytes 0..4, deposit amount in bytes 4..12, as the receipt reads them.
    let receipt_field = |quantity: u32| u128::from(quantity) | u128::from(amount) << 32;
    let order = |capacity: u64, data: Bytes| Cell { capacity, lock: env.limit_order.clone(), type_: Some(env.ickb.clone()), data: data.to_vec() };
    let mint_data = order_data_custom(receipt_field(1), 0, [0; 32], 1i32.to_le_bytes(), (1, 1), (0, 0), 0);
    let hybrid = order(order(0, mint_data.clone()).occupied() + 100 * CKB, mint_data);

    let create = TxSpec {
        inputs: vec![In::new(env.plain(1_000_000 * CKB))],
        outputs: vec![hybrid.clone().into(), env.master().into(), env.deposit(&env.ickb, amount).into(), env.plain(10_000 * CKB).into()],
        ..Default::default()
    };
    let created = create.accept(&mut context, "the order, its master and the deposit the receipt counts should verify");

    let header = gen_header(1_000, GENESIS_AR, 35, 1000, 1000);
    let master = created[1].out_point.clone();
    let match_data = order_data_custom(receipt_field(2), 1, master.tx_hash().as_slice().try_into().expect("hash"), Unpack::<u32>::unpack(&master.index()).to_le_bytes(), (1, 1), (0, 0), 0);
    let matched = TxSpec {
        inputs: vec![created[0].clone().in_block(&header), In::new(env.plain(1_000_000 * CKB))],
        outputs: vec![
            order(hybrid.capacity - 1, match_data).into(),
            env.deposit(&env.ickb, amount).into(),
            env.deposit(&env.ickb, amount).into(),
            env.udt(soft_capped_ickb(amount, GENESIS_AR)).into(),
        ],
        header_deps: vec![header],
    };
    matched.verify(&mut context).expect("a one-shannon match that adds the deposits the higher quantity counts should verify");

    // The same match without the extra deposit leaves the raised receipt unbacked.
    let unbacked = TxSpec { outputs: [&matched.outputs[..2], &matched.outputs[3..]].concat(), ..matched.clone() };
    unbacked.fail(&mut context, ERROR_RECEIPT_MISMATCH);
}
