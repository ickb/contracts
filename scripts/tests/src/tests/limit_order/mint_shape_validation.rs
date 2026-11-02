use super::*;

/// A fresh order at output 0 with this `ckb_min_match_log`, and its master at output 1.
fn mint_with_min_match_log(context: &mut Context, ckb_min_match_log: u8) -> TxSpec {
    let env = Env::new(context);
    let owner_lock = named_always_success_lock(context, b"owner");
    let data = order_data_custom(0, 0, [0u8; 32], 1i32.to_le_bytes(), (1, 1), (0, 0), ckb_min_match_log);
    TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.order(1_500 * CKB, data).into(), env.master().under(&owner_lock).into()],
        ..Default::default()
    }
}

// Mint a fresh pair at the `ckb_min_match_log` encoding boundary, with only output-side validation running, and expect the shape to pass.
#[test]
fn mint_accepts_ckb_min_match_log_64() {
    let mut context = Context::default();
    mint_with_min_match_log(&mut context, 64)
        .verify(&mut context)
        .expect("ckb_min_match_log == 64 should be accepted at the encoding boundary");
}

// Mint the same fresh pair with `ckb_min_match_log` just above the boundary; the creation path executes validation and rejects the shape.
#[test]
fn mint_rejects_ckb_min_match_log_65() {
    let mut context = Context::default();
    mint_with_min_match_log(&mut context, 65).fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CKB_MIN_MATCH_LOG);
}

// Create two would-be master outputs and no order output; the creation path fails as invalid configuration before any duplicate-master-specific conclusion applies.
#[test]
fn two_master_outputs_fail_as_invalid_configuration_not_duplicate_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    TxSpec {
        inputs: vec![In::new(env.plain(1_000 * CKB))],
        outputs: vec![env.master().under(&owner_lock).into(), env.master().under(&owner_lock).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
