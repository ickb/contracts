use super::*;

/// The live two-claim restake: two owned cells claimed and one fresh deposit withdrawn into a new
/// owned cell, whose owner cell goes to `fresh_owner`; `funding` pays and `owner` held the claims.
fn two_claim_restake(
    (ickb_logic, limit_order, owned_owner, dao, xudt): &(Script, Script, Script, Script, Script),
    funding: &Script,
    owner: &Script,
    fresh_owner: &Script,
) -> TxSpec {
    let fresh_deposit_header = rpc_header(
        "0xe35b0c",
        "0x708068b00259a",
        "0xe895f69a88f84b4fda777e8e518d280070b848fbd65c260700a933035c24f108",
    );
    let prior_withdraw_header = rpc_header(
        "0x1010221",
        "0x70803e00029d2",
        "0x4ea3fdead627a751fd5843469f0429008498e3e307a2f70700a3d7f546d1f508",
    );
    let prior_deposit_header = rpc_header(
        "0xde687f",
        "0x708059e0024e6",
        "0x5b1bddaec554e74efcad531d34792800d43522f7ee8c030700112419e9ddfa08",
    );

    let prior_tx = "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c";
    let owned = |index| In {
        header: Some(prior_withdraw_header.clone()),
        since: u64_from_hex("0x200708059e0029d2"),
        ..at(prior_tx, index, live("0xa5e609441a1", owned_owner, Some(dao), "0x7f68de0000000000"))
    };
    TxSpec {
        inputs: vec![
            In {
                header: Some(fresh_deposit_header.clone()),
                ..at(
                    "0xcb91164d9075add730a748150fdd2e32a750cef5051dafed33d1c2aa2cde6806",
                    1,
                    live("0xa6386d7249d", ickb_logic, Some(dao), "0x0000000000000000"),
                )
            },
            at(prior_tx, 6, live("0x13bbb34e26", funding, None, "0x")),
            at(prior_tx, 5, live("0x3663a5200", funding, Some(xudt), "0xa0bfc96d660200000000000000000000")),
            owned(1),
            owned(0),
            at(prior_tx, 4, live("0x248202200", owner, Some(owned_owner), "0xfdffffff")),
            at(prior_tx, 3, live("0x248202200", owner, Some(owned_owner), "0xfdffffff")),
            at(
                prior_tx,
                2,
                live(
                    "0x10186de6e79f",
                    limit_order,
                    Some(xudt),
                    "0x7995e51aa23e00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
            ),
        ],
        outputs: vec![
            live("0xa6386d7249d", owned_owner, Some(dao), "0x0c5be30000000000").into(),
            live(
                "0x251c9490476f",
                limit_order,
                Some(xudt),
                "0xb0905b6c712c00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
            )
            .into(),
            live("0x248202200", fresh_owner, Some(owned_owner), "0xfeffffff").into(),
            live("0x3663a5200", funding, Some(xudt), "0xa9dfe3cd7e0b00000000000000000000").into(),
            live("0x160032266c", funding, None, "0x").into(),
        ],
        header_deps: vec![prior_withdraw_header, prior_deposit_header, fresh_deposit_header],
    }
}

// Replays the live two-claim restake family, but reassigns the fresh owner pair to the attacker, so the later wider phase2 claim should still validate on the stolen path.
#[test]
fn weak_two_claim_live_restake_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let (_, limit_order, _, _, xudt) = &scripts;

    let first = two_claim_restake(&scripts, &user_lock, &user_lock, &attacker_lock);
    let first_tx = replay(
        &mut context,
        &first,
        vec![
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x5500000010000000550000005500000041000000eff31644579c4b2c86d669a691385f9b4725d5a595e9ab73f87ec12e1aeba23a605146dc166dc44af64d34f5e4bde8ba29577a0fb004381de88e1510ccebff9d00"),
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    context
        .verify(&first_tx, MAX_CYCLES)
        .expect("weak two-claim restake should reassign the fresh owner output");

    let fresh_withdraw_header = rpc_header(
        "0x10103e8",
        "0x70805a70029d2",
        "0xd40d0661fe4ba7514c6aad5ca6042900ef8b210b50aef7070068e414c7d3f508",
    );
    let first_hash = first_tx.hash();
    let later = |index: u32| In { out_point: OutPoint::new(first_hash.clone(), index), ..In::new(first.outputs[index as usize].cell.clone()) };
    let second = TxSpec {
        inputs: vec![
            later(4),
            later(3),
            In { header: Some(fresh_withdraw_header.clone()), since: u64_from_hex("0x200708068b0029d2"), ..later(0) },
            later(2),
            later(1),
        ],
        outputs: vec![
            live(
                "0x2f83c2399248",
                limit_order,
                Some(xudt),
                "0xdd5ad75c702300000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
            )
            .into(),
            live("0x3663a5200", &attacker_lock, Some(xudt), "0x7c1568dd7f1400000000000000000000").into(),
            live("0x332dfe9760", &attacker_lock, None, "0x").into(),
        ],
        header_deps: vec![fresh_withdraw_header, first.header_deps[2].clone()],
    };

    let second_tx = replay(
        &mut context,
        &second,
        vec![
            bytes_from_hex("0x5500000010000000550000005500000041000000fc411ffb5ded2280a62a31fbd1750bca07b40c069496a3a9acec2f0664f980ea585f3774a887a5d7dfa3c06e65f9a9ae63be34b964fcb028dd1be0a2040e69a400"),
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    context
        .verify(&second_tx, MAX_CYCLES)
        .expect("reassigned fresh owner output should enable the wider phase2 claim");
}

// Replays the same two-claim restake family with a signed owner group, then swaps a fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_two_claim_live_restake_binds_fresh_owner_output() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-owner");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let spec = two_claim_restake(&scripts, &strong_lock, &weak_lock, &weak_lock);
    let tx = replay(
        &mut context,
        &spec,
        vec![
            Bytes::new(),
            empty_witness(),
            Bytes::new(),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    let tx = sign_tx_by_input_group(tx.as_advanced_builder().cell_dep(secp_data_dep).build(), &privkey, 1, 2);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the two-claim restake");

    let mut outputs: Vec<CellOutput> = tx.outputs().into_iter().collect();
    outputs[2] = cell(u64_from_hex("0x248202200"), &attacker_lock, Some(&scripts.2));
    let tampered_tx = tx.as_advanced_builder().set_outputs(outputs).build();
    fail(&context, &tampered_tx, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}
