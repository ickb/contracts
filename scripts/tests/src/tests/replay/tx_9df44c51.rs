use super::*;

// Replays the live mainnet withdrawal that converts a DAO deposit into an owned_owner pair while refreshing the companion limit_order, so the withdrawal-and-restage shape should verify unchanged.
#[test]
fn mainnet_tx_9df44c51_withdrawal_and_owned_owner_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-withdraw-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xad543d30c47").pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x0000000000000000"),
    );
    let deposit_header = rpc_header(
        "0x11dd50b",
        "0x4a204390035b1",
        "0x04980d2fa73f495894ccb6e29b492a0019b44740d2ec720900ed83b83aff3507",
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let order_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa67ecbf8ac4").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x0000000000000000000000000000000001000000814c44c45e33a496dc30f18131e01bf4b72282be58abd7f0fefff9a95b5c0dbc00000000000000000000000000000000000000000000c16ff2862300a714425582642a0021",
        ),
    );
    let master_input = out_point_from_hex(
        "0x814c44c45e33a496dc30f18131e01bf4b72282be58abd7f0fefff9a95b5c0dbc",
        0,
    );
    context.create_cell_with_out_point(
        master_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(user_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xfc3c6a4e180900000000000000000000"),
    );
    let change_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x180631dd8a2f").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .input(CellInput::new_builder().previous_output(master_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xad543d30c47").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x23c346000").pack())
                .lock(user_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4a221e700").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x226a9a954e53").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0bd51d0100000000"),
                bytes_from_hex("0xffffffff"),
                Bytes::new(),
                bytes_from_hex(
                    "0x194d0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff28623002c93a89882642a0021",
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(vec![Bytes::new(), Bytes::new(), Bytes::new()].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet withdrawal + owned_owner shape should replay locally");
}
