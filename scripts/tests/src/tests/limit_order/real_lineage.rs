use super::*;

/// `real_order` continued into match state at 100 CKB progress, holding `continued` CKB and
/// naming `master` with this ratio.
fn continue_real_order(env: &Env, real_order: In, continued: u64, master: &OutPoint, ckb_to_udt: (u64, u64)) -> TxSpec {
    TxSpec {
        inputs: vec![real_order],
        outputs: vec![env.order(continued, order_data_match(100 * CKB as u128, master, ckb_to_udt)).into()],
        ..Default::default()
    }
}

// Continue a real mint lineage into match state but point it at another real master's outpoint; the match path rejects the arbitrary lineage rewrite.
#[test]
fn real_order_match_cannot_rewrite_master_outpoint_to_an_arbitrary_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (real_order_out_point, _real_master_out_point) =
        build_real_limit_order_and_master_with_capacity(&mut context, owner_lock.clone(), env.helper.clone(), 1_600 * CKB);
    let (_victim_order_out_point, victim_master_out_point) =
        build_real_limit_order_and_master_with_capacity(&mut context, owner_lock.clone(), env.helper.clone(), 1_500 * CKB);

    continue_real_order(&env, live(&context, real_order_out_point), 1_500 * CKB, &victim_master_out_point, (1, 1))
        .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Continue a real mint-state order into its first match state without spending the master cell; the match path accepts the derived metapoint continuation.
#[test]
fn real_limit_order_can_transition_from_mint_to_match_without_consuming_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock, env.helper.clone());

    continue_real_order(&env, live(&context, real_order_out_point), 1_400 * CKB, &real_master_out_point, (1, 1))
        .verify(&mut context)
        .expect("a real minted order should be able to move into a match-state output without consuming its master");
}

// Continue a real mint-state order into match state while changing its ratio info; the match path rejects because the lineage must preserve checked order info.
#[test]
fn real_limit_order_match_rejects_rewriting_order_info() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock, env.helper.clone());

    continue_real_order(&env, live(&context, real_order_out_point), 1_400 * CKB, &real_master_out_point, (2, 1))
        .fail(&mut context, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}
