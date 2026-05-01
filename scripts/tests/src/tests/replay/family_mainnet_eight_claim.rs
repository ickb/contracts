use super::*;

// Replays the live mainnet eight-claim family, but reassigns the freshly created owner outputs to the attacker, so a later phase2 claim should still validate through the stolen branch.
#[test]
fn weak_mainnet_eight_claim_batch_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let weak_lock = named_always_success_lock(&mut context, b"weak-batch");
    let (attacker_privkey, attacker_lock, attacker_secp_data_dep) = secp_lock(&mut context);
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_headers = [
        rpc_header(
            "0xfba66c",
            "0x4ad01f0002fbc",
            "0x9fbfc52bcaf3f45450fb87916da82900d8bd5641b1ee50080002ec946c4a3707",
        ),
        rpc_header(
            "0xfba6dc",
            "0x4ad0260002fbc",
            "0x94870c232c01f5541584192170a829004a9ffda878f3500800956f476c4a3707",
        ),
        rpc_header(
            "0xf7ae28",
            "0x57701af002f09",
            "0xc26333efaae29054fae5e5c24095290025db0c4102762f0800c93eb650f43607",
        ),
        rpc_header(
            "0xf7ae5a",
            "0x57701e1002f09",
            "0x0ca04d8cc7e790549aa202be419529009edf535dd5772f080061c8c69bf43607",
        ),
        rpc_header(
            "0x1004f19",
            "0x70802ae00308b",
            "0x24a5fe69a9956855595128d284be2900c30904672680790800f24015a90e3507",
        ),
    ];
    for header in deposit_headers.iter() {
        context.insert_header(header.clone());
    }

    let deposit_specs = [
        ("0xaac00b02e36", 0usize),
        ("0xaac0158018d", 1usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71849baec", 3usize),
        ("0xab1a870d35c", 4usize),
    ];
    let mut deposit_inputs = Vec::with_capacity(deposit_specs.len());
    for (capacity, header_index) in deposit_specs {
        let out_point = context.create_cell(
            CellOutput::new_builder()
                .capacity(u64_from_hex(capacity).pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            bytes_from_hex("0x0000000000000000"),
        );
        link_cell_to_header(&mut context, &out_point, &deposit_headers[header_index]);
        deposit_inputs.push(out_point);
    }

    let change_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2a31a45188").pack())
            .lock(weak_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(weak_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x4a026c96844b00000000000000000000"),
    );

    let owned_capacities = [
        "0xaac00b02e36",
        "0xaac0158018d",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71849baec",
        "0xab1a870d35c",
    ];
    let owned_datas = [
        "0x6ca6fb0000000000",
        "0xdca6fb0000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x5aaef70000000000",
        "0x194f000100000000",
    ];

    let mut outputs = Vec::new();
    let mut outputs_data = Vec::new();
    for (capacity, data) in owned_capacities.iter().zip(owned_datas.iter()) {
        outputs.push(
            CellOutput::new_builder()
                .capacity(u64_from_hex(capacity).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        );
        outputs_data.push(bytes_from_hex(data));
    }
    for _ in 0..8 {
        outputs.push(
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        );
        outputs_data.push(bytes_from_hex("0xf8ffffff"));
    }
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(weak_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
    );
    outputs_data.push(Bytes::new());
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order)
            .type_(Some(xudt.clone()).pack())
            .build(),
    );
    outputs_data.push(bytes_from_hex(
        "0xd45e0323c20200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fe12a4eaa0ca290021",
    ));
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x142514df48").pack())
            .lock(weak_lock.clone())
            .build(),
    );
    outputs_data.push(Bytes::new());

    let first_tx = TransactionBuilder::default()
        .inputs(
            deposit_inputs
                .iter()
                .cloned()
                .chain([change_input.clone(), udt_input.clone()])
                .map(|out_point| CellInput::new_builder().previous_output(out_point).build())
                .collect::<Vec<_>>(),
        )
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .header_dep(deposit_headers[0].hash())
        .header_dep(deposit_headers[1].hash())
        .header_dep(deposit_headers[2].hash())
        .header_dep(deposit_headers[3].hash())
        .header_dep(deposit_headers[4].hash())
        .build();

    let first_tx = context.complete_tx(first_tx);
    context
        .verify_tx(&first_tx, MAX_CYCLES)
        .expect("weak authorization should reassign the mainnet batch fresh owner outputs");

    let batch_withdraw_header = rpc_header(
        "0x103a436",
        "0x511038b00311b",
        "0xe8527c922537b9559c17b476e1cd290056cedd92ed7d9508008a16eefae93407",
    );
    let first_hash = first_tx.hash();
    let fresh_owned = OutPoint::new(first_hash.clone(), 0);
    let fresh_owner = OutPoint::new(first_hash, 8);
    context.create_cell_with_out_point(
        fresh_owned.clone(),
        first_tx.outputs().get(0).expect("fresh owned output"),
        bytes_from_hex("0x6ca6fb0000000000"),
    );
    context.create_cell_with_out_point(
        fresh_owner.clone(),
        first_tx.outputs().get(8).expect("fresh owner output"),
        bytes_from_hex("0xf8ffffff"),
    );
    link_cell_to_header(&mut context, &fresh_owned, &batch_withdraw_header);

    let claim_capacity = dao_maximum_withdraw_capacity(
        &first_tx.outputs().get(0).expect("fresh owned cell"),
        withdrawal_request_data(u64_from_hex("0xfba66c")).len(),
        11_725_662_591_646_544,
        11_766_842_287_986_588,
    );
    let witness = header_dep_index_witness(1);
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(fresh_owned)
                .since(u64_from_hex("0x2004ad01f0003124").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(fresh_owner).build())
        .output(
            CellOutput::new_builder()
                .capacity(claim_capacity.pack())
                .lock(attacker_lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .witness(witness.pack())
        .witness(empty_witness().pack())
        .cell_dep(attacker_secp_data_dep)
        .header_dep(batch_withdraw_header.hash())
        .header_dep(deposit_headers[0].hash())
        .build();

    let claim_tx = sign_tx_by_input_group(context.complete_tx(claim_tx), &attacker_privkey, 1, 1);
    context
        .verify_tx(&claim_tx, MAX_CYCLES)
        .expect("reassigned mainnet batch owner output should allow a later phase2 claim");
}

// Replays the same eight-claim family with one signed owner group, then swaps one fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_mainnet_eight_claim_batch_binds_fresh_owner_outputs() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-batch");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_headers = [
        rpc_header(
            "0xfba66c",
            "0x4ad01f0002fbc",
            "0x9fbfc52bcaf3f45450fb87916da82900d8bd5641b1ee50080002ec946c4a3707",
        ),
        rpc_header(
            "0xfba6dc",
            "0x4ad0260002fbc",
            "0x94870c232c01f5541584192170a829004a9ffda878f3500800956f476c4a3707",
        ),
        rpc_header(
            "0xf7ae28",
            "0x57701af002f09",
            "0xc26333efaae29054fae5e5c24095290025db0c4102762f0800c93eb650f43607",
        ),
        rpc_header(
            "0xf7ae5a",
            "0x57701e1002f09",
            "0x0ca04d8cc7e790549aa202be419529009edf535dd5772f080061c8c69bf43607",
        ),
        rpc_header(
            "0x1004f19",
            "0x70802ae00308b",
            "0x24a5fe69a9956855595128d284be2900c30904672680790800f24015a90e3507",
        ),
    ];
    for header in deposit_headers.iter() {
        context.insert_header(header.clone());
    }

    let deposit_specs = [
        ("0xaac00b02e36", 0usize),
        ("0xaac0158018d", 1usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71806dfc1", 2usize),
        ("0xaa71849baec", 3usize),
        ("0xab1a870d35c", 4usize),
    ];
    let mut deposit_inputs = Vec::with_capacity(deposit_specs.len());
    for (capacity, header_index) in deposit_specs {
        let out_point = context.create_cell(
            CellOutput::new_builder()
                .capacity(u64_from_hex(capacity).pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            bytes_from_hex("0x0000000000000000"),
        );
        link_cell_to_header(&mut context, &out_point, &deposit_headers[header_index]);
        deposit_inputs.push(out_point);
    }

    let change_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2a31a45188").pack())
            .lock(strong_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(strong_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x4a026c96844b00000000000000000000"),
    );

    let owned_capacities = [
        "0xaac00b02e36",
        "0xaac0158018d",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71849baec",
        "0xab1a870d35c",
    ];
    let owned_datas = [
        "0x6ca6fb0000000000",
        "0xdca6fb0000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x5aaef70000000000",
        "0x194f000100000000",
    ];

    let mut outputs = Vec::new();
    let mut outputs_data = Vec::new();
    for (capacity, data) in owned_capacities.iter().zip(owned_datas.iter()) {
        outputs.push(
            CellOutput::new_builder()
                .capacity(u64_from_hex(capacity).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        );
        outputs_data.push(bytes_from_hex(data));
    }
    for _ in 0..8 {
        outputs.push(
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(weak_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        );
        outputs_data.push(bytes_from_hex("0xf8ffffff"));
    }
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(strong_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
    );
    outputs_data.push(Bytes::new());
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order)
            .type_(Some(xudt.clone()).pack())
            .build(),
    );
    outputs_data.push(bytes_from_hex(
        "0xd45e0323c20200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fe12a4eaa0ca290021",
    ));
    outputs.push(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x142514df48").pack())
            .lock(strong_lock.clone())
            .build(),
    );
    outputs_data.push(Bytes::new());

    let tx = TransactionBuilder::default()
        .inputs(
            deposit_inputs
                .iter()
                .cloned()
                .chain([change_input.clone(), udt_input.clone()])
                .map(|out_point| CellInput::new_builder().previous_output(out_point).build())
                .collect::<Vec<_>>(),
        )
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .header_dep(deposit_headers[0].hash())
        .header_dep(deposit_headers[1].hash())
        .header_dep(deposit_headers[2].hash())
        .header_dep(deposit_headers[3].hash())
        .header_dep(deposit_headers[4].hash())
        .witnesses(
            vec![
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                Bytes::new(),
                empty_witness(),
                Bytes::new(),
            ]
            .pack(),
        )
        .cell_dep(secp_data_dep)
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 8, 2);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the mainnet batch");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            tx.outputs().get(0).expect("fresh owned output"),
            tx.outputs().get(1).expect("fresh owned output 1"),
            tx.outputs().get(2).expect("fresh owned output 2"),
            tx.outputs().get(3).expect("fresh owned output 3"),
            tx.outputs().get(4).expect("fresh owned output 4"),
            tx.outputs().get(5).expect("fresh owned output 5"),
            tx.outputs().get(6).expect("fresh owned output 6"),
            tx.outputs().get(7).expect("fresh owned output 7"),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(attacker_lock)
                .type_(Some(owned_owner).pack())
                .build(),
            tx.outputs().get(9).expect("fresh owner output 1"),
            tx.outputs().get(10).expect("fresh owner output 2"),
            tx.outputs().get(11).expect("fresh owner output 3"),
            tx.outputs().get(12).expect("fresh owner output 4"),
            tx.outputs().get(13).expect("fresh owner output 5"),
            tx.outputs().get(14).expect("fresh owner output 6"),
            tx.outputs().get(15).expect("fresh owner output 7"),
            tx.outputs().get(16).expect("master output"),
            tx.outputs().get(17).expect("order output"),
            tx.outputs().get(18).expect("change output"),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}
