use super::*;

// Melt one real pair and remint a new sparse negative-distance pair in the same transaction; the old master path closes cleanly and the new pair remains meltable.
#[test]
fn can_atomically_melt_and_remint_with_negative_distance_and_filler() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let (old_order_out_point, old_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), env.helper.clone());

    let remint = TxSpec {
        inputs: vec![
            live(&context, old_order_out_point),
            live(&context, old_master_out_point),
            In::new(env.plain(100 * CKB)),
        ],
        outputs: vec![
            env.master().under(&owner_lock).into(),
            plain_under(&filler_lock, 100 * CKB).into(),
            env.order(1_500 * CKB, order_data_mint(0, -2, (1, 1))).into(),
        ],
        ..Default::default()
    };
    let reminted = commit(
        &mut context,
        &remint,
        "limit_order should allow canceling one pair while minting a new sparse negative-distance pair in the same tx",
    );

    TxSpec {
        inputs: vec![reminted[2].clone(), reminted[0].clone(), In::new(env.plain(1_000 * CKB))],
        outputs: vec![plain_under(&owner_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("the reminted sparse pair should remain a valid melt target");
}
