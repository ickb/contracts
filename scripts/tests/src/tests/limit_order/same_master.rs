use super::*;

// Continue one match-shaped order into two outputs that both cite the same master; the match path rejects the duplicated metapoint fan-out.
#[test]
fn match_rejects_two_outputs_sharing_one_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);
    TxSpec {
        inputs: vec![In::new(env.order_unused(1_500 * CKB, order_data_match(0, &master, (1, 1)))), env.funding_input()],
        outputs: vec![
            env.order_unused(1_400 * CKB, order_data_match(50 * CKB as u128, &master, (1, 1))).into(),
            env.order_unused(100 * CKB, order_data_match(10 * CKB as u128, &master, (1, 1))).into(),
        ],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Spend two independent match-shaped inputs that already point at the same master; verification rejects the shared metapoint collision on inputs.
#[test]
fn same_master_collision_on_inputs_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);
    TxSpec {
        inputs: vec![
            In::new(env.order_unused(1_500 * CKB, order_data_match(0, &master, (1, 1)))),
            In::new(env.order_unused(1_400 * CKB, order_data_match(0, &master, (1, 1)))),
        ],
        outputs: vec![env.plain(2_900 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Attempt to spend the exact same master input twice; transaction validation blocks the duplicate input before any contract duplicate-master branch can execute.
#[test]
fn duplicate_master_input_shape_is_blocked_before_script_invariants() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (_real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), env.helper.clone());
    let master = live(&context, real_master_out_point);
    // Duplicate inputs are rejected by transaction-level validation before the contract can reach
    // its internal DuplicatedMaster branch, so there is no stable limit_order error code to assert.
    let tx = TxSpec {
        inputs: vec![master.clone(), master],
        outputs: vec![plain(&owner_lock, 400 * CKB).into()],
        ..Default::default()
    }
    .build(&mut context);
    // The duplicated input is the point, so the node's spent-twice rule overlaps by construction.
    context
        .verify_where_node_also_rejects(&tx, MAX_CYCLES, "spent twice")
        .expect_err("duplicating a master input should be blocked before a reachable DuplicatedMaster path");
}
