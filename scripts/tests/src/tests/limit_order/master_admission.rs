use super::*;

// Mint a real pair whose master uses a foreign non-empty-args lock; creation passes because that master lock does not execute, but melt fails once it does.
#[test]
fn mint_accepts_master_with_unspendable_foreign_lock() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let poisoned_lock = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(1_500 * CKB, &limit_order, Some(&helper_type)))
        .output(
            cell(occupied_capacity(&poisoned_lock, &limit_order, 0), &poisoned_lock, Some(&limit_order)),
        )
        .outputs_data(vec![order_data_mint(0, 1, (1, 1)), Bytes::new()].pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("limit_order mint accepts a master whose lock never executed on creation");

    let tx_hash = create_tx.hash();
    let order_out_point = OutPoint::new(tx_hash.clone(), 0);
    let master_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        order_out_point.clone(),
        create_tx.outputs().get(0).expect("order output"),
        order_data_mint(0, 1, (1, 1)),
    );
    context.create_cell_with_out_point(
        master_out_point.clone(),
        create_tx.outputs().get(1).expect("master output"),
        Bytes::new(),
    );

    let melt_tx = TransactionBuilder::default()
        .input(input(order_out_point))
        .input(input(master_out_point))
        .output(cell(1_700 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    fail(&context, &melt_tx, ERROR_NOT_EMPTY_ARGS);
}

// Mint a real pair whose master uses `ickb_logic` with empty args; creation still skips the master lock, and melt later fails when that foreign script executes.
#[test]
fn mint_accepts_master_with_empty_args_ickb_logic_lock_and_strands_on_spend() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let benign_foreign_lock = ickb_logic_script(&mut context);
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(1_500 * CKB, &limit_order, Some(&helper_type)))
        .output(
            cell(occupied_capacity(&benign_foreign_lock, &limit_order, 0), &benign_foreign_lock, Some(&limit_order)),
        )
        .outputs_data(vec![order_data_mint(0, 1, (1, 1)), Bytes::new()].pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("limit_order mint accepts an empty-args ickb_logic master lock at creation");

    let tx_hash = create_tx.hash();
    let order_out_point = OutPoint::new(tx_hash.clone(), 0);
    let master_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        order_out_point.clone(),
        create_tx.outputs().get(0).expect("order output"),
        order_data_mint(0, 1, (1, 1)),
    );
    context.create_cell_with_out_point(
        master_out_point.clone(),
        create_tx.outputs().get(1).expect("master output"),
        Bytes::new(),
    );

    let melt_tx = TransactionBuilder::default()
        .input(input(order_out_point))
        .input(input(master_out_point))
        .output(cell(1_700 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    fail(&context, &melt_tx, ERROR_SCRIPT_MISUSE);
}
