use super::*;

// An occupied-capacity-valid fulfilled CKB->UDT cell has no capacity left to decrease, so a continuation fails the outer match-shape check.
#[test]
fn fulfilled_ckb_to_udt_shape_fails_as_invalid_match() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 0), &limit_order, Some(&helper_type)),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order))
        .output(
            cell(deposit_capacity(&limit_order, &helper_type, 89, 0), &limit_order, Some(&helper_type)),
        )
        .output_data(order_data_match(1, &master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Forge a fulfilled UDT->CKB-shaped cell and try to continue it; this shape never reaches the inner fulfilled guard and instead fails the outer match validation.
#[test]
fn fulfilled_udt_to_ckb_shape_cannot_reach_guard_and_fails_as_invalid_match() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        cell(100 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order))
        .input(input(funding_input))
        .outputs(vec![
            cell(deposit_capacity(&limit_order, &helper_type, 89, 1_520 * CKB), &limit_order, Some(&helper_type)),
            cell(80 * CKB, &funding_lock, None),
        ])
        .outputs_data(
            vec![
                // The contract's fulfilled-order guard for UDT -> CKB sits behind `i.udt > o.udt`,
                // so a zero-UDT input can only be observed failing the outer match-shape check.
                order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_INVALID_MATCH);
}
