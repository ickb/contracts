use super::*;
use ckb_testtool::ckb_types::core::HeaderView;

mod dao_crosswiring;
mod foreign_dao_wrapping;
mod foreign_owner_lock_boundaries;
mod live_claim_rotation;
mod melt_pairing_and_witness;
mod mixed_asset_crosswiring;
mod owner_distance_encoding;
mod pair_formation;
mod phase1_poisoning;
mod positive_distance_claims;
mod script_misuse;
mod weak_lock_output_rebinding;
mod withdrawal_amount_and_headers;

/// The block of a deposit committed at `number`, at the genesis accumulated rate.
fn deposit_header(number: u64) -> HeaderView {
    gen_header(number, GENESIS_AR, 35, 1000, 1000)
}

/// The block of a withdrawal request committed at `number`, in epoch 575 at the synthetic withdraw rate.
fn withdraw_header(number: u64) -> HeaderView {
    gen_header(number, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100)
}

/// A cell under `lock` with nothing else.
fn plain(lock: &Script, capacity: u64) -> Cell {
    Cell { capacity, lock: lock.clone(), type_: None, data: vec![] }
}

/// The 1,000 CKB cell `funding_cell` creates, as a wallet adds one to fund the outputs.
fn funding(env: &Env) -> In {
    In::new(plain(&env.funding, 1_000 * CKB))
}

/// An iCKB xUDT cell under `lock` holding `amount`, at its occupied capacity.
fn udt(env: &Env, lock: &Script, amount: u128) -> Cell {
    Cell { capacity: occupied_capacity(lock, &env.xudt, 16), lock: lock.clone(), ..env.udt(amount) }
}

/// A NervosDAO withdrawal request locked by Owned Owner, for a deposit committed at `deposit_number`.
fn owned(env: &Env, capacity: u64, deposit_number: u64) -> Cell {
    Cell { capacity, lock: env.owned_owner.clone(), type_: Some(env.dao.clone()), data: withdrawal_request_data(deposit_number).to_vec() }
}

/// An owner cell under `lock`, `distance` cells away from its owned cell, at its occupied capacity.
fn owner(env: &Env, lock: &Script, distance: i32) -> Cell {
    Cell { capacity: occupied_capacity(lock, &env.owned_owner, 4), lock: lock.clone(), type_: Some(env.owned_owner.clone()), data: owner_distance_data(distance).to_vec() }
}

/// `cell` as a live input committed in `header`.
fn mined(cell: Cell, header: &HeaderView) -> In {
    In { header: Some(header.clone()), ..In::new(cell) }
}

/// `cell` as a live input at output `index` of transaction `tx_hash`. Owned Owner pairs input cells
/// through these out points, so a phase 2 input keeps the one its pair was created at.
fn at(cell: Cell, tx_hash: &Byte32, index: usize) -> In {
    In { out_point: OutPoint::new(tx_hash.clone(), index as u32), ..In::new(cell) }
}

/// Verifies `spec` and returns its outputs as live inputs at their out points, for the next step.
fn accept(context: &mut Context, spec: &TxSpec, expect: &str) -> Vec<In> {
    let tx = spec.build(context);
    context.verify(&tx, MAX_CYCLES).expect(expect);
    spec.outputs.iter().enumerate().map(|(i, o)| at(o.cell.clone(), &tx.hash(), i)).collect()
}

/// `request` as a phase 2 claim input: withdrawn in `withdraw_header`, spent at `since`, with its
/// deposit header at header dep `deposit_index`.
fn claim(request: In, withdraw_header: &HeaderView, since: u64, deposit_index: u64) -> In {
    In { header: Some(withdraw_header.clone()), since, witness: header_dep_index_witness(deposit_index), ..request }
}

/// The most a phase 2 claim of `request` may take, from a genesis-rate deposit.
fn claim_capacity(request: &Cell) -> u64 {
    dao_maximum_withdraw_capacity(&request.output(), request.data.len(), GENESIS_AR, SYNTHETIC_WITHDRAW_AR)
}

/// Phase 1 of withdrawing iCKB deposits of `amounts`, the i-th committed at block 1554 + i: the
/// deposits as inputs, their owned requests as outputs at the same indices, and their headers.
/// Callers add the xUDT burnt and the owner cells.
fn withdrawals(env: &Env, amounts: &[u64]) -> TxSpec {
    let mut spec = TxSpec::default();
    for (number, &amount) in (1554..).zip(amounts) {
        let header = deposit_header(number);
        let deposit = env.deposit(&env.ickb, amount);
        spec.outputs.push(owned(env, deposit.capacity, number).into());
        spec.inputs.push(mined(deposit, &header));
        spec.header_deps.push(header);
    }
    spec
}

/// Phase 1 of one iCKB withdrawal: the 1,000 CKB deposit committed at block 1554 and the xUDT
/// under `owner_lock` burnt for it, becoming the owned request and its owner cell at distance -1.
fn withdrawal(env: &Env, owner_lock: &Script) -> TxSpec {
    let mut spec = withdrawals(env, &[1_000 * CKB]);
    spec.inputs.push(In::new(udt(env, owner_lock, u128::from(1_000 * CKB))));
    spec.outputs.push(owner(env, owner_lock, -1).into());
    spec
}

/// Phase 2 of a pair created at outputs 0 and 1 of the transaction hashed `[tx_byte; 32]`: the
/// owned request of 123,456,780,000 shannons for a deposit at block 1554, withdrawn at block
/// 2,000,610 and claimed with its deposit header at header dep 1, beside `owner`. Callers add outputs.
fn melt(env: &Env, tx_byte: u8, owner: Cell) -> TxSpec {
    let tx_hash = [tx_byte; 32].pack();
    let withdraw_header = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![
            claim(at(owned(env, 123_456_780_000, 1554), &tx_hash, 0), &withdraw_header, 0x2003e800000002f4, 1),
            at(owner, &tx_hash, 1),
        ],
        outputs: vec![],
        header_deps: vec![withdraw_header, deposit_header(1554)],
    }
}
