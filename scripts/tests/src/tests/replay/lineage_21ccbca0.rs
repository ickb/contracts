use super::*;

// Replays the live testnet phase1 funding tx that mints a DAO deposit, receipt, limit_order master, and order in one step, so the full creation shape should verify unchanged.
#[test]
fn testnet_tx_21ccbca0_phase1_and_limit_order_creation_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-user-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let funding_capacity = [
        "0xa5e5d45d23e",
        "0x283baec00",
        "0x23c346000",
        "0x1bd2464cc2",
        "0x51dde56f7306",
    ]
    .into_iter()
    .map(u64_from_hex)
    .sum::<u64>();
    let funding_input = context.create_cell(
        cell(funding_capacity, &user_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![
            cell(u64_from_hex("0xa5e5d45d23e"), &ickb_logic, Some(&dao)),
            cell(u64_from_hex("0x283baec00"), &user_lock, Some(&ickb_logic)),
            cell(u64_from_hex("0x23c346000"), &user_lock, Some(&limit_order)),
            cell(u64_from_hex("0x1bd2464cc2"), &limit_order, Some(&xudt)),
            cell(u64_from_hex("0x51dde56f7306"), &user_lock, None),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x010000003ec083745c0a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300dc1703b6fa8a28000000000000000000000000000000000021",
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(
            bytes_from_hex(
                "0x6900000010000000690000006900000055000000550000001000000055000000550000004100000002022854075348bb8af50b68bb2781113d192ea5b146975f146e4f809e9e7eec58e9f149708ccc2a824288f1337b06c1a74d49bf4c6fcff64cad4a67bca1ab3000",
            )
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live testnet phase1 + limit_order creation shape should replay locally");
}

// Replays the live follow-up phase2 plus limit_order tx for the same testnet lineage without changing protocol cells, so the mixed receipt-consumption shape should still verify.
#[test]
fn testnet_tx_a4a8dc3d_mixed_phase2_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-user-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let order_input = cell(u64_from_hex("0x1bd2464cc2"), &limit_order, Some(&xudt));
    let master_input = cell(u64_from_hex("0x23c346000"), &user_lock, Some(&limit_order));
    let change_input = cell(u64_from_hex("0x51dde56f7306"), &user_lock, None);
    let receipt_input = cell(u64_from_hex("0x283baec00"), &user_lock, Some(&ickb_logic));
    let order_out_point = out_point_from_hex("0x21ccbca021f03816c7057bdfca5a60da9e2280ca1c2530affa608df9c63c7954", 3);
    let master_out_point = out_point_from_hex("0x21ccbca021f03816c7057bdfca5a60da9e2280ca1c2530affa608df9c63c7954", 2);
    let change_out_point = out_point_from_hex("0x21ccbca021f03816c7057bdfca5a60da9e2280ca1c2530affa608df9c63c7954", 4);
    let receipt_out_point = out_point_from_hex("0x21ccbca021f03816c7057bdfca5a60da9e2280ca1c2530affa608df9c63c7954", 1);
    context.create_cell_with_out_point(
        order_out_point.clone(),
        order_input,
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300dc1703b6fa8a28000000000000000000000000000000000021",
        ),
    );
    context.create_cell_with_out_point(master_out_point.clone(), master_input, Bytes::new());
    context.create_cell_with_out_point(change_out_point.clone(), change_input, Bytes::new());
    context.create_cell_with_out_point(
        receipt_out_point.clone(),
        receipt_input,
        bytes_from_hex("0x010000003ec083745c0a0000"),
    );
    let receipt_header = rpc_header(
        "0xde6557",
        "0x70802760024e6",
        "0xd2048e9c9114e74ec3403a4327792800265d7cdcba760307007b46166bcdfa08",
    );
    link_cell_to_header(&mut context, &receipt_out_point, &receipt_header);

    let tx = TransactionBuilder::default()
        .input(input(order_out_point))
        .input(input(master_out_point))
        .input(input(change_out_point))
        .input(input(receipt_out_point))
        .outputs(vec![
            cell(u64_from_hex("0xa5e5e84cc7e"), &ickb_logic, Some(&dao)),
            cell(u64_from_hex("0x283baec00"), &user_lock, Some(&ickb_logic)),
            cell(u64_from_hex("0x23c346000"), &user_lock, Some(&limit_order)),
            cell(u64_from_hex("0x1bd1075282"), &limit_order, Some(&xudt)),
            cell(u64_from_hex("0x372261400"), &user_lock, Some(&xudt)),
            cell(u64_from_hex("0x477c16037c64"), &user_lock, None),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x010000007ebac2755c0a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff28623005b9a4395ff8a28000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x084e6b4e180900000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(bytes_from_hex("0x10000000100000001000000010000000").pack())
        .witness(
            bytes_from_hex(
                "0x690000001000000069000000690000005500000055000000100000005500000055000000410000003745bbb5c0ec2a17e54b98a2f50b3badb5234aecfe69454f39dd3b29bf41e6f81644695bb3b19b8e89e12482455f992c9d5fabb906e28f27f0d5540cdf21095400",
            )
            .pack(),
        )
        .header_dep(receipt_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live testnet mixed phase2 + limit_order shape should replay locally");
}

// Replays the live batched withdrawal that turns three deposits into owned_owner pairs plus a continuing limit_order, so the historical batched claim shape should verify unchanged.
#[test]
fn testnet_tx_088a1019_batched_withdrawal_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-owner-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit1_out_point = out_point_from_hex("0xa4a8dc3d3226cc73bb568bfbe0b9bb568bcd0d373404ff80ad88aea93c335900", 0);
    let deposit2_out_point = out_point_from_hex("0x0775023511f378eec5e8720fa6c4adfeaeaa9bbac7477af061ce812f836ebb81", 0);
    let deposit3_out_point = out_point_from_hex("0x0f2fb555f86e93a755c8a242b71dc173824c4309c0b6d1913e44955beaf5352d", 5);
    let order_out_point = out_point_from_hex("0xb97b7026be8e9cccba8422a003866c35997391d69163df3fb2c1b8441dd1f6db", 1);
    let master_out_point = out_point_from_hex("0xffef829fc3a19246d19f4458f66c25228c3acde7e9a2613d37f66dd2ef956fd9", 0);
    let udt_out_point = out_point_from_hex("0xffef829fc3a19246d19f4458f66c25228c3acde7e9a2613d37f66dd2ef956fd9", 2);
    context.create_cell_with_out_point(
        deposit1_out_point.clone(),
        cell(u64_from_hex("0xa5e5e84cc7e"), &ickb_logic, Some(&dao)),
        bytes_from_hex("0x0000000000000000"),
    );
    context.create_cell_with_out_point(
        deposit2_out_point.clone(),
        cell(u64_from_hex("0xa5e5ec78350"), &ickb_logic, Some(&dao)),
        bytes_from_hex("0x0000000000000000"),
    );
    context.create_cell_with_out_point(
        deposit3_out_point.clone(),
        cell(u64_from_hex("0xa5e609441a1"), &ickb_logic, Some(&dao)),
        bytes_from_hex("0x0000000000000000"),
    );
    context.create_cell_with_out_point(
        udt_out_point.clone(),
        cell(u64_from_hex("0x3663a5200"), &user_lock, Some(&xudt)),
        bytes_from_hex("0x56b5feac722100000000000000000000"),
    );
    context.create_cell_with_out_point(
        order_out_point.clone(),
        cell(u64_from_hex("0x34a6e480ce1c"), &limit_order, Some(&xudt)),
        bytes_from_hex(
            "0x0000000000000000000000000000000001000000ffef829fc3a19246d19f4458f66c25228c3acde7e9a2613d37f66dd2ef956fd900000000000000000000000000000000000000000000c16ff2862300058a01b24b76280021",
        ),
    );
    context.create_cell_with_out_point(
        master_out_point.clone(),
        cell(u64_from_hex("0x230489e00"), &user_lock, Some(&limit_order)),
        Bytes::new(),
    );

    let deposit1_header = rpc_header(
        "0xde6685",
        "0x70803a40024e6",
        "0x3ed1f3cb902ce74e6520f2102c792800ae26145e077f030700585dae9dcdfa08",
    );
    let deposit2_header = rpc_header(
        "0xde66c6",
        "0x70803e50024e6",
        "0xf8bb6201bb31e74eebab9e192d7928009237ffa0d080030700490fc546d4fa08",
    );
    let deposit3_header = rpc_header(
        "0xde687f",
        "0x708059e0024e6",
        "0x5b1bddaec554e74efcad531d34792800d43522f7ee8c030700112419e9ddfa08",
    );
    link_cell_to_header(&mut context, &deposit1_out_point, &deposit1_header);
    link_cell_to_header(&mut context, &deposit2_out_point, &deposit2_header);
    link_cell_to_header(&mut context, &deposit3_out_point, &deposit3_header);

    let tx = TransactionBuilder::default()
        .input(input(deposit1_out_point))
        .input(input(deposit2_out_point))
        .input(input(deposit3_out_point))
        .input(input(udt_out_point))
        .input(input(order_out_point))
        .input(input(master_out_point))
        .outputs(vec![
            cell(u64_from_hex("0xa5e5e84cc7e"), &owned_owner, Some(&dao)),
            cell(u64_from_hex("0xa5e5ec78350"), &owned_owner, Some(&dao)),
            cell(u64_from_hex("0xa5e609441a1"), &owned_owner, Some(&dao)),
            cell(u64_from_hex("0x248202200"), &user_lock, Some(&owned_owner)),
            cell(u64_from_hex("0x248202200"), &user_lock, Some(&owned_owner)),
            cell(u64_from_hex("0x248202200"), &user_lock, Some(&owned_owner)),
            cell(u64_from_hex("0x230489e00"), &user_lock, Some(&limit_order)),
            cell(u64_from_hex("0x4a221e700"), &limit_order, Some(&xudt)),
            cell(u64_from_hex("0x349ed038bab7"), &user_lock, None),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x8566de0000000000"),
                bytes_from_hex("0xc666de0000000000"),
                bytes_from_hex("0x7f68de0000000000"),
                bytes_from_hex("0xfdffffff"),
                bytes_from_hex("0xfdffffff"),
                bytes_from_hex("0xfdffffff"),
                Bytes::new(),
                bytes_from_hex(
                    "0x84bab2c1290600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fdbbc4ea1b84280021",
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(bytes_from_hex("0x10000000100000001000000010000000").pack())
        .witness(bytes_from_hex("0x10000000100000001000000010000000").pack())
        .witness(bytes_from_hex("0x10000000100000001000000010000000").pack())
        .witness(
            bytes_from_hex(
                "0x5500000010000000550000005500000041000000a9b94cb7c4da3ac43a21ca6efacf65c3a9cfc29b0dbd13cdc54cd5c53afc70522e7dc5205301b61748b24ce2abf074390bfe0de19cf181184d6341a8ee5c475100",
            )
            .pack(),
        )
        .header_dep(deposit1_header.hash())
        .header_dep(deposit2_header.hash())
        .header_dep(deposit3_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live testnet batched withdrawal + limit_order shape should replay locally");
}
