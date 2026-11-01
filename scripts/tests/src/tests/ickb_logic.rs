use super::*;
use ckb_testtool::ckb_types::core::HeaderView;

mod creation_blind_spots;
mod dao_phase2_batching;
mod deposit_bounds;
mod economic_precision;
mod mixed_flow_composition;
mod multi_quantity_receipts;
mod phase2_headers;
mod phase2_recipient_binding;
mod phase2_sibling_classification;
mod phase2_xudt_output_data;
mod receipt_encoding;
mod receipt_matching;
mod receiptless_deposit_blind_spot;
mod soft_cap;
mod xudt_amount_limits;
mod xudt_owner_witness_fallback;




/// A phase2 mint of `minted` xUDT from `receipt`, created in a genesis-rate block.
fn mint(env: &Env, receipt: Cell, minted: u128, funded: bool) -> TxSpec {
    mint_at(env, receipt, &gen_header(0, GENESIS_AR, 0, 0, 1), minted, funded)
}

/// A phase2 mint of `minted` xUDT from `receipt`, created in `header`'s block, which the header deps
/// list. `funded` adds the wallet's funding input; rejection cases omit it, as the scripts fail first.
fn mint_at(env: &Env, receipt: Cell, header: &HeaderView, minted: u128, funded: bool) -> TxSpec {
    let mut inputs = vec![In::new(receipt).in_block(header)];
    if funded {
        inputs.push((env).funding_input());
    }
    TxSpec { inputs, outputs: vec![env.udt(minted).into()], header_deps: vec![header.clone()] }
}

/// The phase 1 withdrawal request for `deposit`, created in block `number`, under `lock`.
fn request(deposit: &Cell, lock: &Script, number: u64) -> Cell {
    Cell { lock: lock.clone(), data: withdrawal_request_data(number).to_vec(), ..deposit.clone() }
}

