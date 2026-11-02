use super::*;

/// Phase 1 burning the xUDT under `owner_lock` for the 1,000 CKB deposit committed at block 1554,
/// funded by `funding_input`, with `outputs` laid out by the caller.
fn create(env: &Env, funding_input: In, owner_lock: &Script, outputs: Vec<Out>) -> TxSpec {
    let header = deposit_header(1554);
    TxSpec {
        inputs: vec![
            funding_input,
            In::new((env).udt(u128::from(1_000 * CKB)).under(owner_lock)),
            In::new(env.deposit(&env.ickb, 1_000 * CKB)).in_block(&header),
        ],
        outputs,
        header_deps: vec![header],
    }
}

/// The phase 2 claim of the accepted pair at `owner` and `request`, paying its exact capacity to `owner_lock`.
fn exact_claim(owner_lock: &Script, owner: In, request: In) -> TxSpec {
    let withdraw_header = withdraw_header(2_000_610);
    let claimed = claim_capacity(&request.cell);
    TxSpec {
        inputs: vec![claim(request, &withdraw_header, 0x2003e800000002f4, 1), owner],
        outputs: vec![plain(owner_lock, claimed).into()],
        header_deps: vec![withdraw_header, deposit_header(1554)],
    }
}

// Scenario: the owner cell is placed immediately before the owned cell and points forward with distance `1`.
// Expectation: both phase 1 creation and the later phase 2 DAO claim succeed.
#[test]
fn adjacent_positive_distance_pair_can_complete_phase2_claim() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let deposit = env.deposit(&env.ickb, 1_000 * CKB);
    let outputs = vec![
        plain(&filler_lock, 100 * CKB).into(),
        env.owner(1).under(&owner_lock).into(),
        owned(&env, deposit.capacity, 1554).into(),
    ];
    let create = create(&env, In::new(plain(&env.funding, 200 * CKB)), &owner_lock, outputs);
    let live = create.accept(&mut context, "owned_owner should accept an adjacent positive-distance phase1 pair");

    exact_claim(&owner_lock, live[1].clone(), live[2].clone())
        .verify(&mut context)
        .expect("an adjacent positive-distance pair should remain spendable in DAO phase2");
}

// Scenario: phase 1 creates a sparse positive-distance pair with a filler output between owner and owned cells.
// Expectation: phase 1 accepts the layout, and the exact DAO claim capacity remains spendable in phase 2.
#[test]
fn sparse_positive_distance_pair_can_complete_phase2_claim_at_exact_capacity() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let funding_lock = always_success_lock(&mut context);
    let deposit = env.deposit(&env.ickb, 1_000 * CKB);
    let outputs = vec![
        env.owner(2).under(&owner_lock).into(),
        plain(&filler_lock, 100 * CKB).into(),
        owned(&env, deposit.capacity, 1554).into(),
    ];
    let create = create(&env, In::new(plain(&funding_lock, 200 * CKB)), &owner_lock, outputs);
    let live = create.accept(&mut context, "owned_owner should accept a sparse positive-distance pair when DAO index rules are still satisfied");

    exact_claim(&owner_lock, live[0].clone(), live[2].clone())
        .verify(&mut context)
        .expect("a sparse positive-distance pair should remain spendable in DAO phase2 at the exact claim capacity");
}

// Scenario: phase 1 consumes two noncontiguous inputs protected by the same secp lock, then phase 2 spends the signed owner cell.
// Expectation: both signed transactions verify and the sparse Owned Owner pair completes its DAO claim.
#[test]
fn secp_protected_sparse_pair_can_complete_signed_phase2_claim() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        cell(100 * CKB, &owner_lock, None),
        Bytes::new(),
    );
    let filler_input = context.create_cell(
        cell(100 * CKB, &filler_lock, None),
        Bytes::new(),
    );
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .input(input(filler_input))
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(100 * CKB, &filler_lock, None),
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
        ])
        .outputs_data(
            vec![
                owner_distance_data(2),
                Bytes::new(),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .witness(empty_witness().pack())
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .cell_dep(secp_data_dep.clone())
        .build();
    let create_tx = sign_tx_by_input_indices(context.complete_tx(create_tx), &privkey, &[0, 3]);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("signed phase1 creation with a noncontiguous secp group should verify");

    let owner_out_point = seed_verified_output(&mut context, &create_tx, 0, owner_distance_data(2));
    let owned_out_point = seed_verified_output(&mut context, &create_tx, 2, withdrawal_request_data(1554));
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let exact_capacity = dao_maximum_withdraw_capacity(
        &create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64)
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(exact_capacity, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(header_dep_index_witness(1).pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .build();
    let claim_tx = sign_tx_by_input_group(context.complete_tx(claim_tx), &privkey, 1, 1);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("signed secp-protected owner should complete the phase2 DAO claim");
}
