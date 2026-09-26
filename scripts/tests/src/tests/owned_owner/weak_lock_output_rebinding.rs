use super::*;

// Recipient binding depends on the user lock, not Owned Owner itself.
// With a weak owner lock, phase 1 can redirect the owner cell to a different later claimant even though the DAO request itself is unchanged.
#[test]
fn weak_lock_can_reassign_withdrawal_owner_output() {
    let mut context = Context::default();
    let weak_lock = always_success_lock(&mut context);
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        weak_lock,
        deposit_amount,
        1554,
    );

    let owned_output = cell(deposit_total_capacity, &owned_owner, Some(&dao));
    let owner_output = cell(occupied_capacity(&attacker_lock, &owned_owner, 4), &attacker_lock, Some(&owned_owner));

    let tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
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

    let deposit_amount = 1_000 * CKB;
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
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .witness(Bytes::new().pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .header_dep(deposit_header.hash())
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 1, 1);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("signed withdrawal request should verify");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao_script(&mut context))),
            cell(occupied_capacity(&attacker_lock, &owned_owner_script(&mut context), 4), &attacker_lock, Some(&owned_owner_script(&mut context))),
        ])
        .build();
    fail(&context, &tampered_tx, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
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

    let amount1 = 1_000 * CKB;
    let amount2 = 1_200 * CKB;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = create_deposit(&mut context, total1, &ickb_logic, &dao);
    let deposit2 = create_deposit(&mut context, total2, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let strong_udt = create_udt(&mut context, &owner_lock, &xudt, u128::from(amount1));
    let weak_udt = create_udt(&mut context, &weak_lock, &xudt, u128::from(amount2));

    let tx = TransactionBuilder::default()
        .input(input(deposit1))
        .input(input(deposit2))
        .input(input(strong_udt))
        .input(input(weak_udt))
        .outputs(vec![
            cell(total1, &owned_owner, Some(&dao)),
            cell(total2, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
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
        .verify(&tx, MAX_CYCLES)
        .expect("mixed strong+weak withdrawal batch should verify when signed");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            cell(total1, &owned_owner_script(&mut context), Some(&dao_script(&mut context))),
            cell(total2, &owned_owner_script(&mut context), Some(&dao_script(&mut context))),
            cell(occupied_capacity(&attacker_lock, &owned_owner_script(&mut context), 4), &attacker_lock, Some(&owned_owner_script(&mut context))),
            cell(occupied_capacity(&attacker_lock, &owned_owner_script(&mut context), 4), &attacker_lock, Some(&owned_owner_script(&mut context))),
        ])
        .build();
    fail(&context, &tampered_tx, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
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

    let amount1 = 1_000 * CKB;
    let amount2 = 1_200 * CKB;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = create_deposit(&mut context, total1, &ickb_logic, &dao);
    let deposit2 = create_deposit(&mut context, total2, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let weak_udt_1 = create_udt(&mut context, &weak_lock_1, &xudt, u128::from(amount1));
    let weak_udt_2 = create_udt(&mut context, &weak_lock_2, &xudt, u128::from(amount2));

    let tx = TransactionBuilder::default()
        .input(input(deposit1))
        .input(input(deposit2))
        .input(input(weak_udt_1))
        .input(input(weak_udt_2))
        .outputs(vec![
            cell(total1, &owned_owner, Some(&dao)),
            cell(total2, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&attacker_lock, &owned_owner, 4), &attacker_lock, Some(&owned_owner)),
            cell(occupied_capacity(&attacker_lock, &owned_owner, 4), &attacker_lock, Some(&owned_owner)),
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
        .verify(&tx, MAX_CYCLES)
        .expect("all-weak withdrawal inputs can reassign both withdrawal owner outputs");
}
