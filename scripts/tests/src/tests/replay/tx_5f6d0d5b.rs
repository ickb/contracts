use super::*;

// Replays the live mainnet tx that combines an owned_owner claim, a fresh deposit and receipt, and a continuing limit_order match, so the combined transition should verify unchanged.
#[test]
fn mainnet_tx_5f6d0d5b_claim_deposit_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-5f6d-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let change_input_out_point = out_point_from_hex(
        "0xd48e2f9ec9dd567b6d77f7e25bcd0f0cbfca6444240f9d4720d2bbc61cd99e4a",
        1,
    );
    context.create_cell_with_out_point(
        change_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xce777783bd6").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input_out_point = out_point_from_hex(
        "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
        2,
    );
    context.create_cell_with_out_point(
        udt_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x85026831280500000000000000000000"),
    );

    let owned_input_out_point = out_point_from_hex(
        "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
        0,
    );
    let owned_input = CellOutput::new_builder()
        .capacity(u64_from_hex("0xab4adefc030").pack())
        .lock(owned_owner.clone())
        .type_(Some(dao.clone()).pack())
        .build();
    context.create_cell_with_out_point(
        owned_input_out_point.clone(),
        owned_input.clone(),
        bytes_from_hex("0x43d9020100000000"),
    );

    let owner_input_out_point = out_point_from_hex(
        "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
        1,
    );
    context.create_cell_with_out_point(
        owner_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xffffffff"),
    );

    let order_input_out_point = out_point_from_hex(
        "0xf8aed34a7b086cc50b92bbb292045da54473d1c5e50b0937788f6b84adb22170",
        4,
    );
    context.create_cell_with_out_point(
        order_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x710eb256272").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230054043b34e65b2a000000000000000000000000000000000021",
        ),
    );

    let withdraw_header = rpc_header(
        "0x11b4ad3",
        "0x50d0501003531",
        "0x8b3e331079c70158fafaada83c3c2a00282092094f7e590900477a3182df3507",
    );
    link_cell_to_header(&mut context, &owned_input_out_point, &withdraw_header);
    let deposit_header = rpc_header(
        "0x102d943",
        "0x5f301630030fa",
        "0xbd51139d8b84a65574ecbf4c52ca290057c2dd0e1ef38e0800d5521830ef3407",
    );
    context.insert_header(deposit_header.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(change_input_out_point).build())
        .input(CellInput::new_builder().previous_output(udt_input_out_point).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_out_point)
                .since(u64_from_hex("0x2005f30163003532").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input_out_point).build())
        .input(CellInput::new_builder().previous_output(order_input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xfe564f2176").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xad543dd639c").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x277cf2a00").pack())
                .lock(user_lock.clone())
                .type_(Some(ickb_logic.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x12f6704164db").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0x00c002da17050000000000000000000001000000f8aed34a7b086cc50b92bbb292045da54473d1c5e50b0937788f6b84adb22170030000000000c16ff286230054043b34e65b2a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x010000009c511b5bd30a0000"),
                bytes_from_hex("0x85426557100000000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(header_dep_index_witness(1).pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet claim plus fresh deposit and limit_order shape should replay locally");
}
