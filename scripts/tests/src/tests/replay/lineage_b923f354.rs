use super::*;

// Replays the live pure mainnet limit_order match step without mutating the order family, so the order should stay in match mode and verify unchanged.
#[test]
fn mainnet_tx_b923f354_live_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-match-lock");
    let (_ickb_logic, limit_order, xudt) = ickb_logic_limit_order_and_xudt_scripts(&mut context);

    let change_input = out_point_from_hex(
        "0xe473654b3c0cb2fbd245051bc42befbe6c0bada95b983160e95382901d02895a",
        2,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        cell(u64_from_hex("0x17ca7ad60207"), &user_lock, None),
        Bytes::new(),
    );
    let udt_input = out_point_from_hex(
        "0xe473654b3c0cb2fbd245051bc42befbe6c0bada95b983160e95382901d02895a",
        1,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        cell(u64_from_hex("0x3663a5200"), &user_lock, Some(&xudt)),
        bytes_from_hex("0x4e83e3e7250500000000000000000000"),
    );
    let order_input = out_point_from_hex(
        "0x04f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df40",
        1,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        cell(u64_from_hex("0x4a221e700"), &limit_order, Some(&xudt)),
        bytes_from_hex(
            "0xff2222085e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff28623000a1338d4d7672a0021",
        ),
    );

    let tx = TransactionBuilder::default()
        .input(input(change_input))
        .input(input(udt_input))
        .input(input(order_input))
        .outputs(vec![
            cell(u64_from_hex("0x74deeef1ad"), &limit_order, Some(&xudt)),
            cell(u64_from_hex("0x3663a5200"), &user_lock, Some(&xudt)),
            cell(u64_from_hex("0x175a3e080786"), &user_lock, None),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0x000000000000000000000000000000000100000004f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df4000000000000000000000000000000000000000000000c16ff28623000a1338d4d7672a0021",
                ),
                bytes_from_hex("0x4da605f0830500000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(
            bytes_from_hex(
                "0x5500000010000000550000005500000041000000b3fbc543b97dc328921e75f0324715fc2035eb40b977c212c9358c88457848204304782a57cbc96ab273302c51f191d7d580d7861ea32b821ce7cd79001935b901",
            )
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet pure limit_order match shape should replay locally");
}

// Replays the live follow-up melt for that matched order without changing the referenced master or funding set, so the historical melt shape should verify unchanged.
#[test]
fn mainnet_tx_3d26da3b_limit_order_melt_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-melt-lock");
    let (_ickb_logic, limit_order, xudt) = ickb_logic_limit_order_and_xudt_scripts(&mut context);

    let order_out_point = out_point_from_hex(
        "0xb923f3541646cd6d6247b82b04d875a5b957ab98b52d41c16c1159d9bfb70353",
        0,
    );
    context.create_cell_with_out_point(
        order_out_point.clone(),
        cell(u64_from_hex("0x74deeef1ad"), &limit_order, Some(&xudt)),
        bytes_from_hex(
            "0x000000000000000000000000000000000100000004f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df4000000000000000000000000000000000000000000000c16ff28623000a1338d4d7672a0021",
        ),
    );

    let master_out_point = out_point_from_hex(
        "0x04f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df40",
        0,
    );
    context.create_cell_with_out_point(
        master_out_point.clone(),
        cell(u64_from_hex("0x230489e00"), &user_lock, Some(&limit_order)),
        Bytes::new(),
    );

    let change_out_point = out_point_from_hex(
        "0x04f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df40",
        2,
    );
    context.create_cell_with_out_point(
        change_out_point.clone(),
        cell(u64_from_hex("0x1e4155fb4f"), &user_lock, None),
        Bytes::new(),
    );

    let funding_out_point_1 = out_point_from_hex(
        "0x0d691577731b5eca73036de93afd710c9541a35cead603c9d7378aaefae6d628",
        0,
    );
    context.create_cell_with_out_point(
        funding_out_point_1.clone(),
        cell(u64_from_hex("0x5256bc3fd7"), &user_lock, None),
        Bytes::new(),
    );

    let funding_out_point_2 = out_point_from_hex(
        "0x87680af9f3ded98e15079da3aaf08f8f1674130b56bd3325b81bb03a932371b2",
        0,
    );
    context.create_cell_with_out_point(
        funding_out_point_2.clone(),
        cell(u64_from_hex("0xbbe5fd5f6"), &user_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(order_out_point))
        .input(input(master_out_point))
        .input(input(funding_out_point_1))
        .input(input(change_out_point))
        .input(input(funding_out_point_2))
        .output(cell(u64_from_hex("0xf365a998f9"), &user_lock, None))
        .output_data(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(
            bytes_from_hex(
                "0x55000000100000005500000055000000410000002632318cc63dc488a4bf3182d29ca0a0d436c4bf98782eca7c8c26f0dadc17212ff1937d165b4dff91e4ab63af62f5d6b572b1177b93f80600ff24ab6739d2e200",
            )
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet limit_order melt shape should replay locally");
}
