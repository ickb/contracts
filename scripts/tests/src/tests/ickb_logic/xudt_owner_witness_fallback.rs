use super::*;

// Build a plain funding tx that tries to mint xUDT by supplying the xUDT owner-script witness in `output_type`: no live iCKB owner-mode path exists in the cell set, so the fallback witness does not authorize minting and verification fails.
#[test]
fn xudt_owner_script_output_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let witness = witness_with_output_type(xudt_owner_script_witness(env.ickb.clone()));
    let spec = TxSpec {
        inputs: vec![In { witness, ..In::new(env.plain(500 * CKB)) }],
        outputs: vec![env.udt(1).into()],
        ..Default::default()
    };
    spec.fail(&mut context, ERROR_XUDT_AMOUNT);
}

// Build a plain xUDT self-transfer that increases the amount while supplying the xUDT owner-script witness in `input_type`: without a live iCKB owner-mode route, the fallback witness still cannot authorize minting, so verification fails.
#[test]
fn xudt_owner_script_input_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let witness = witness_with_input_type(xudt_owner_script_witness(env.ickb.clone()));
    let spec = TxSpec {
        inputs: vec![In { witness, ..In::new(env.udt(1)) }],
        outputs: vec![env.udt(2).into()],
        ..Default::default()
    };
    spec.fail(&mut context, ERROR_XUDT_AMOUNT);
}
