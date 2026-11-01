use super::*;

/// A phase2 mint of a 1,000 CKB receipt whose xUDT output holds `data`, at its occupied capacity.
fn mint_holding(context: &mut Context, data: Vec<u8>, funded: bool) -> TxSpec {
    let env = Env::new(context);
    let amount = 1_000 * CKB;
    let mut spec = mint(&env, env.receipt(1, amount), u128::from(amount), funded);
    spec.outputs[0] = (Cell { data, ..env.udt(0) }).sized().into();
    spec
}

// Build a phase2 mint whose xUDT output data starts with a correct 16-byte amount and appends extra bytes: the parser should read only the amount prefix, so the mint still passes.
#[test]
fn phase2_mint_accepts_xudt_data_with_trailing_bytes() {
    let mut context = Context::default();
    let mut data = udt_data(u128::from(1_000 * CKB)).to_vec();
    data.push(0xaa);
    let spec = mint_holding(&mut context, data, true);
    spec.verify(&mut context).expect("phase2 mint should accept xudt data with trailing bytes");
}

// Build a phase2 mint whose xUDT output data truncates the amount to 8 bytes: this violates the minimum xUDT amount encoding, so verification fails.
#[test]
fn phase2_mint_rejects_short_xudt_output_data() {
    let mut context = Context::default();
    let data = truncated_bytes(udt_data(u128::from(1_000 * CKB)), 8).to_vec();
    mint_holding(&mut context, data, false).fail(&mut context, ERROR_ENCODING);
}

// Build a phase2 mint whose xUDT output data is empty: there is no encoded amount at all, so the output data shape is invalid and verification fails.
#[test]
fn phase2_mint_rejects_zero_length_xudt_output_data() {
    let mut context = Context::default();
    mint_holding(&mut context, vec![], false).fail(&mut context, ERROR_ENCODING);
}
