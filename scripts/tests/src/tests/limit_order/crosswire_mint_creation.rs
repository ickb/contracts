use super::*;

// Create two mint pairs with crosswired distances: the script still pairs each order with exactly one master, so creation passes, the swapped master can melt, and the intuitive pairing fails.
#[test]
fn mint_crosswire_swaps_order_masters() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let owner2_lock = named_always_success_lock(&mut context, b"owner2");
    let create = TxSpec {
        inputs: vec![In::new(env.plain(4_000 * CKB))],
        outputs: vec![
            env.order(1_500 * CKB, order_data_mint(0, 3, (1, 1))).into(),
            env.master().under(&owner1_lock).into(),
            env.order(1_500 * CKB, order_data_mint(0, -1, (1, 1))).into(),
            env.master().under(&owner2_lock).into(),
        ],
        ..Default::default()
    };
    let created = commit(&mut context, &create, "cross-wired master assignment should pass deployed mint validation");
    let (order1, master1, order2) = (&created[0], &created[1], &created[2]);

    TxSpec {
        inputs: vec![order2.clone(), master1.clone(), In::new(env.plain(1_000 * CKB))],
        outputs: vec![plain_under(&owner1_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("master1 is bound to order2 when mint distances are cross-wired");

    TxSpec {
        inputs: vec![order1.clone(), master1.clone()],
        outputs: vec![plain_under(&owner2_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Create the same crosswire through sparse output spacing and negative distances; limit_order runs at mint and accepts the crosswired distances, and the far paired master remains the valid melt path.
#[test]
fn sparse_far_distance_limit_order_crosswire_still_rebinds_master_assignment() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let owner2_lock = named_always_success_lock(&mut context, b"owner2");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let filler = || Out::from(plain_under(&filler_lock, 100 * CKB));

    let mut outputs = vec![env.order(1_500 * CKB, order_data_mint(0, 11, (1, 1))).into()];
    outputs.extend((0..8).map(|_| filler()));
    outputs.extend([
        env.order(1_500 * CKB, order_data_mint(0, 3, (1, 1))).into(),
        filler(),
        env.master().under(&owner2_lock).into(),
        env.master().under(&owner1_lock).into(),
    ]);
    let create = TxSpec { inputs: vec![In::new(env.plain(8_000 * CKB))], outputs, ..Default::default() };
    let created = commit(&mut context, &create, "sparse far-distance crosswire should still pass deployed mint validation");
    let (order1, master2, master1) = (&created[0], &created[11], &created[12]);

    TxSpec {
        inputs: vec![order1.clone(), master2.clone(), In::new(env.plain(1_000 * CKB))],
        outputs: vec![plain_under(&owner2_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("far-distance sparse layout still binds order1 to owner2's master");

    TxSpec {
        inputs: vec![order1.clone(), master1.clone()],
        outputs: vec![plain_under(&owner1_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// A master's type script runs at mint and pairs every order with exactly one master: one order per
// master passes, two orders on one master fail, a master without an order fails, and an order
// pointing at a cell that is no master fails.
#[test]
fn mint_pairs_each_order_with_exactly_one_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let order = |distance: i32| Out::from(env.order_unused(1_500 * CKB, order_data_mint(0, distance, (1, 1))));
    let master = || Out::from(env.master().under(&owner_lock));
    let mint = |outputs: Vec<Out>| TxSpec {
        inputs: vec![In::new(env.plain(1_000 * CKB)), In::new(env.plain(1_000 * CKB))],
        outputs,
        ..Default::default()
    };

    mint(vec![order(1), master()]).verify(&mut context).expect("one order with its own master mints");
    mint(vec![order(2), order(1), master()]).fail(&mut context, ERROR_LIMIT_ORDER_SAME_MASTER);
    mint(vec![order(1), master(), master()]).fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
    let plain = Out::from(plain_under(&owner_lock, 100 * CKB));
    mint(vec![order(1), master(), order(1), plain]).fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
