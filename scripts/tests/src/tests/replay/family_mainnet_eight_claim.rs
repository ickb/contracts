use super::*;

/// The live mainnet eight-claim batch: eight deposits withdrawn into owned cells, each paired with
/// a fresh owner cell under `owner`, funded by `funding` and refreshing the batch's limit order.
fn eight_claim_batch(
    (ickb_logic, limit_order, owned_owner, dao, xudt): &(Script, Script, Script, Script, Script),
    funding: &Script,
    owner: &Script,
) -> TxSpec {
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
    // Each deposit's capacity, the header it was deposited in, and its withdrawal request data.
    let deposits = [
        ("0xaac00b02e36", 0usize, "0x6ca6fb0000000000"),
        ("0xaac0158018d", 1usize, "0xdca6fb0000000000"),
        ("0xaa71806dfc1", 2usize, "0x28aef70000000000"),
        ("0xaa71806dfc1", 2usize, "0x28aef70000000000"),
        ("0xaa71806dfc1", 2usize, "0x28aef70000000000"),
        ("0xaa71806dfc1", 2usize, "0x28aef70000000000"),
        ("0xaa71849baec", 3usize, "0x5aaef70000000000"),
        ("0xab1a870d35c", 4usize, "0x194f000100000000"),
    ];

    let mut inputs: Vec<In> = deposits
        .iter()
        .map(|&(capacity, header, _)| In {
            header: Some(deposit_headers[header].clone()),
            ..In::new(live(capacity, ickb_logic, Some(dao), "0x0000000000000000"))
        })
        .collect();
    inputs.push(In::new(live("0x2a31a45188", funding, None, "0x")));
    inputs.push(In::new(live("0x372261400", funding, Some(xudt), "0x4a026c96844b00000000000000000000")));

    let mut outputs: Vec<Out> = deposits.iter().map(|&(capacity, _, data)| live(capacity, owned_owner, Some(dao), data).into()).collect();
    outputs.extend((0..8).map(|_| live("0x2540be400", owner, Some(owned_owner), "0xf8ffffff").into()));
    outputs.push(live("0x23c346000", funding, Some(limit_order), "0x").into());
    outputs.push(
        live(
            "0x4a221e700",
            limit_order,
            Some(xudt),
            "0xd45e0323c20200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fe12a4eaa0ca290021",
        )
        .into(),
    );
    outputs.push(live("0x142514df48", funding, None, "0x").into());

    TxSpec { inputs, outputs, header_deps: deposit_headers.to_vec() }
}

// Replays the live mainnet eight-claim family, but reassigns the freshly created owner outputs to the attacker, so a later phase2 claim should still validate through the stolen branch.
#[test]
fn weak_mainnet_eight_claim_batch_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let weak_lock = named_always_success_lock(&mut context, b"weak-batch");
    let (attacker_privkey, attacker_lock, attacker_secp_data_dep) = secp_lock(&mut context);
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let first = eight_claim_batch(&scripts, &weak_lock, &attacker_lock);
    let first_tx = replay(&mut context, &first, vec![]);
    context
        .verify(&first_tx, MAX_CYCLES)
        .expect("weak authorization should reassign the mainnet batch fresh owner outputs");

    let batch_withdraw_header = rpc_header(
        "0x103a436",
        "0x511038b00311b",
        "0xe8527c922537b9559c17b476e1cd290056cedd92ed7d9508008a16eefae93407",
    );
    let first_hash = first_tx.hash();
    let fresh_owned = first.outputs[0].cell.clone();
    let claim_capacity = dao_maximum_withdraw_capacity(
        &fresh_owned.output(),
        withdrawal_request_data(u64_from_hex("0xfba66c")).len(),
        11_725_662_591_646_544,
        11_766_842_287_986_588,
    );
    let claim = TxSpec {
        inputs: vec![
            In {
                out_point: OutPoint::new(first_hash.clone(), 0),
                header: Some(batch_withdraw_header.clone()),
                since: u64_from_hex("0x2004ad01f0003124"),
                witness: header_dep_index_witness(1),
                ..In::new(fresh_owned)
            },
            In { out_point: OutPoint::new(first_hash, 8), witness: empty_witness(), ..In::new(first.outputs[8].cell.clone()) },
        ],
        outputs: vec![Cell { capacity: claim_capacity, lock: attacker_lock, type_: None, data: vec![] }.into()],
        header_deps: vec![batch_withdraw_header, first.header_deps[0].clone()],
    };

    let claim_tx = claim.build(&mut context).as_advanced_builder().cell_dep(attacker_secp_data_dep).build();
    let claim_tx = sign_tx_by_input_group(claim_tx, &attacker_privkey, 1, 1);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("reassigned mainnet batch owner output should allow a later phase2 claim");
}

// Replays the same eight-claim family with one signed owner group, then swaps one fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_mainnet_eight_claim_batch_binds_fresh_owner_outputs() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-batch");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let owned_owner = scripts.2.clone();

    let mut spec = eight_claim_batch(&scripts, &strong_lock, &weak_lock);
    // The signer fills the lock of its group's first witness.
    spec.inputs[8].witness = empty_witness();
    let tx = spec.build(&mut context).as_advanced_builder().cell_dep(secp_data_dep).build();
    let tx = sign_tx_by_input_group(tx, &privkey, 8, 2);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the mainnet batch");

    let mut outputs: Vec<CellOutput> = tx.outputs().into_iter().collect();
    outputs[8] = cell(u64_from_hex("0x2540be400"), &attacker_lock, Some(&owned_owner));
    let tampered_tx = tx.as_advanced_builder().set_outputs(outputs).build();
    fail(&context, &tampered_tx, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}
