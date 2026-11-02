use super::*;

/// Mints an order with its master under `master_lock`, then builds the melt of that pair.
fn mint_then_melt(context: &mut Context, master_lock: &Script) -> TxSpec {
    let env = Env::new(context);
    let mint = TxSpec {
        inputs: vec![In::new(env.plain(2_000 * CKB))],
        outputs: vec![env.order(1_500 * CKB, order_data_mint(0, 1, (1, 1))).into(), env.master().under(master_lock).into()],
        ..Default::default()
    };
    let mut inputs = mint.accept(context, "the mint should verify because a master lock does not execute on creation");
    inputs.push(env.funding_input());
    TxSpec {
        inputs,
        outputs: vec![env.plain(1_700 * CKB).into()],
        ..Default::default()
    }
}

// Mint a real pair whose master uses a foreign non-empty-args lock; creation passes because that master lock does not execute, but melt fails once it does.
#[test]
fn mint_accepts_master_with_unspendable_foreign_lock() {
    let mut context = Context::default();
    let poisoned_lock = data1_script(&mut context, "ickb_logic", Bytes::from(vec![1]));
    mint_then_melt(&mut context, &poisoned_lock).fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Mint a real pair whose master uses `ickb_logic` with empty args; creation still skips the master lock, and melt later fails when that foreign script executes.
#[test]
fn mint_accepts_master_with_empty_args_ickb_logic_lock_and_strands_on_spend() {
    let mut context = Context::default();
    let benign_foreign_lock = ickb_logic_script(&mut context);
    mint_then_melt(&mut context, &benign_foreign_lock).fail(&mut context, ERROR_SCRIPT_MISUSE);
}
