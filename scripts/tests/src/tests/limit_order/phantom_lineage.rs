use super::*;

/// A live mint-state order whose distance names a master at output 5 that never existed.
fn phantom_order(env: &Env) -> In {
    In::new(env.order(1_500 * CKB, order_data_mint(0, 5, (1, 1))))
}

// Create a mint-shaped output lock with no real master cell behind its distance; creation passes because the transaction holds no master cell, so limit_order never runs, and the order's own lock does not execute on output.
#[test]
fn phantom_mint_output_can_be_created() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.order(1_500 * CKB, order_data_mint(0, 5, (1, 1))).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("phantom order creation should bypass limit_order validation");
}

// Continue that phantom mint into match state using the derived metapoint only; the match path accepts even though no real master input exists.
#[test]
fn phantom_mint_lineage_can_enter_match_without_real_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let phantom = phantom_order(&env);
    let phantom_master = OutPoint::new(phantom.out_point.tx_hash(), 5);
    TxSpec {
        inputs: vec![phantom],
        outputs: vec![env.order(1_400 * CKB, order_data_match(100 * CKB as u128, &phantom_master, (1, 1))).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("a phantom mint-shaped order should be able to transition into match state without any real master");
}

// Continue a phantom mint into match state but rewrite its metapoint to an unrelated fake master; the match path rejects the lineage rebind.
#[test]
fn phantom_mint_lineage_cannot_rebind_to_an_arbitrary_fake_match_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);
    TxSpec {
        inputs: vec![phantom_order(&env)],
        outputs: vec![env.order(1_400 * CKB, order_data_match(100 * CKB as u128, &fake_master, (1, 1))).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Continue a phantom mint into match state while changing its ratio info; the match path still enforces same-order info and rejects the rewrite.
#[test]
fn phantom_limit_order_match_still_requires_same_order_info() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let phantom = phantom_order(&env);
    let phantom_master = OutPoint::new(phantom.out_point.tx_hash(), 5);
    TxSpec {
        inputs: vec![phantom],
        outputs: vec![env.order(1_400 * CKB, order_data_match(100 * CKB as u128, &phantom_master, (2, 1))).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}

// Try to melt a phantom mint without any master input; the melt path rejects because no matching master lock/type pair is present.
#[test]
fn phantom_limit_order_cannot_be_melted_without_a_master_input() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    TxSpec {
        inputs: vec![phantom_order(&env)],
        outputs: vec![env.plain(1_500 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Try to melt a phantom mint against an unrelated real master cell; the melt path rejects because the derived metapoint does not match that master.
#[test]
fn phantom_limit_order_cannot_be_melted_with_an_unrelated_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    TxSpec {
        inputs: vec![phantom_order(&env), In::new(env.master().under(&owner_lock)), env.funding_input()],
        outputs: vec![plain(&owner_lock, 1_700 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Create a lock-only output that already carries match data and a fake master; creation succeeds because output locks do not execute.
#[test]
fn lock_only_limit_order_output_can_be_created_with_match_order_data() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);
    TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.order(1_500 * CKB, order_data_match(0, &fake_master, (1, 1))).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("lock-only output can be created with MatchOrderData");
}
