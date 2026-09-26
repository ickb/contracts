use super::*;

// Build a valid receipt-to-xUDT phase2 mint but add an unrelated output lock that reuses ickb_logic code with non-empty args: sibling classification still executes that shape, so the whole tx fails on the empty-args invariant.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(200 * CKB, &poisoned_lock, None),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), Bytes::new()].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}

// Build a valid phase2 mint but add another output whose lock is the iCKB xUDT script itself: this sibling looks like script misuse rather than a normal foreign output, so verification fails.
#[test]
fn ickb_xudt_shaped_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(capacity_for_data(16), &xudt, None),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), udt_data(0)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_SCRIPT_MISUSE);
}

// Build a valid phase2 mint but add another output whose lock is the DAO script with deposit-shaped data: that sibling is an impossible lock shape the classifier rejects, so the transaction fails as misuse.
#[test]
fn dao_deposit_shaped_output_lock_poisons_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(capacity_for_data(8), &dao, None),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), dao_deposit_data()].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_SCRIPT_MISUSE);
}

// Build a valid phase2 mint plus a second output whose lock uses the same xUDT code under different args: the sibling is foreign to iCKB accounting, so the real mint should still pass.
#[test]
fn foreign_xudt_output_lock_is_ignored() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);
    let foreign_owner = named_always_success_lock(&mut context, b"foreign-owner");
    let foreign_xudt = xudt_script(&mut context, &foreign_owner);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(capacity_for_data(16), &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(capacity_for_data(16), &foreign_xudt, None),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), udt_data(0)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("same xudt code with different args should stay outside iCKB classification");
}

// Build a valid phase2 mint plus a second typed output that uses the same xUDT code under different args: the accounting boundary is script hash plus args, so this foreign xUDT type should be ignored and the tx passes.
#[test]
fn foreign_xudt_type_output_is_ignored() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);
    let foreign_owner = named_always_success_lock(&mut context, b"foreign-owner");
    let foreign_xudt = xudt_script(&mut context, &foreign_owner);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(capacity_for_data(16), &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(occupied_capacity(&funding_lock, &foreign_xudt, 16), &funding_lock, Some(&foreign_xudt)),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), udt_data(0)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("same xudt code with different args should stay outside iCKB UDT accounting");
}

// Build a valid phase2 mint plus a DAO-locked output that carries withdrawal-request data instead of deposit data: this sibling should stay outside deposit classification, so the mint still passes.
#[test]
fn withdrawal_request_shaped_dao_output_lock_is_ignored() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(capacity_for_data(8), &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(capacity_for_data(8), &dao, None),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), withdrawal_request_data(1554)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("non-zero dao-shaped lock outputs should remain outside deposit classification");
}

// Build a valid phase2 mint plus an output that closely resembles an iCKB deposit but uses non-empty ickb_logic args: the plausible deposit shape still violates the args invariant, so verification fails.
#[test]
fn deposit_shaped_non_empty_args_output_poisons_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let poisoned_lock = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    let xudt = xudt_script(&mut context, &ickb_logic);
    let dao = dao_script(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let extra_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(extra_input))
        .outputs(vec![
            cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)),
            cell(deposit_capacity(&poisoned_lock, &dao, 8, deposit_amount), &poisoned_lock, Some(&dao)),
        ])
        .outputs_data(vec![udt_data(u128::from(deposit_amount)), dao_deposit_data()].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}

// Build a valid phase2 mint but include a second receipt input whose type script reuses ickb_logic code with non-empty args: sibling input classification sees an invalid receipt-shaped peer, so the whole mint fails on the args check.
#[test]
fn non_empty_args_receipt_sibling_poisons_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let poisoned_receipt_type = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    let xudt = xudt_script(&mut context, &ickb_logic);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let poisoned_receipt = create_receipt(&mut context, &funding_lock, &poisoned_receipt_type, 1, deposit_amount);
    insert_header_for_cell(&mut context, &poisoned_receipt, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .input(input(poisoned_receipt))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .header_dep(receipt_header)
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}
