use super::*;

// Start from a forged match-shaped cell that already names a fake master, then keep continuing it; the match path accepts because each step stays self-consistent.
#[test]
fn fake_match_lineage_can_keep_advancing_without_real_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);
    let step = |input: In, capacity: u64, matched: u64| TxSpec {
        inputs: vec![input],
        outputs: vec![env.order(capacity, order_data_match(u128::from(matched), &fake_master, (1, 1))).into()],
        ..Default::default()
    };

    let initial_order = In::new(env.order(1_500 * CKB, order_data_match(0, &fake_master, (1, 1))));
    let first_match = commit(
        &mut context,
        &step(initial_order, 1_400 * CKB, 100 * CKB),
        "a forged match-shaped order should survive one valid-looking match transition without any real master",
    );
    step(first_match[0].clone(), 1_300 * CKB, 200 * CKB)
        .verify(&mut context)
        .expect("the fake match-shaped lineage should stay reusable across multiple later match transitions without any real master");
}

// Forge a foreign-token order with unrelated pricing that points at a real master, melt through that master, then show the legitimate order is stranded.
#[test]
fn foreign_token_fake_order_with_arbitrary_info_can_strand_real_order() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mint = TxSpec {
        inputs: vec![In::new(env.plain(1_700 * CKB))],
        outputs: vec![env.order(1_500 * CKB, order_data_mint(0, 1, (1, 1))).into(), env.master().under(&owner_lock).into()],
        ..Default::default()
    };
    let minted = commit(&mut context, &mint, "the real order and occupied-capacity-valid master should mint");
    let (real_order, real_master) = (minted[0].clone(), minted[1].clone());

    let foreign_token = named_always_success_lock(&mut context, b"foreign-token");
    let master_index: u32 = real_master.out_point.index().unpack();
    let fake_data = order_data_custom(
        123,
        1,
        real_master.out_point.tx_hash().as_slice().try_into().expect("master tx hash"),
        master_index.to_le_bytes(),
        (7, 11),
        (0, 0),
        42,
    );
    let phantom_order = Cell { type_: Some(foreign_token), ..env.order(1_500 * CKB, fake_data) };
    TxSpec {
        inputs: vec![In::new(phantom_order), real_master, In::new(env.plain(1_000 * CKB))],
        outputs: vec![plain_under(&owner_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("a foreign-token fake order with arbitrary pricing should melt against the referenced real master");

    TxSpec {
        inputs: vec![real_order],
        outputs: vec![env.plain(1_500 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
