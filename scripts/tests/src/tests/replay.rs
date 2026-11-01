use super::*;

mod family_attacker_limit_order;
mod family_mainnet_eight_claim;
mod family_mixed_restake;
mod family_two_claim_restake;
mod lineage_21ccbca0;
mod lineage_30f8ed8a;
mod lineage_86d442e2;
mod lineage_b866945c;
mod lineage_b923f354;
mod tx_5f6d0d5b;
mod tx_9df44c51;

/// A cell with capacity and data in the hex the chain reports them in.
fn live(capacity: &str, lock: &Script, type_: Option<&Script>, data: &str) -> Cell {
    Cell { capacity: u64_from_hex(capacity), lock: lock.clone(), type_: type_.cloned(), data: bytes_from_hex(data).to_vec() }
}

/// The cell as an input at its on-chain out point.
fn at(tx_hash: &str, index: u32, cell: Cell) -> In {
    In { out_point: out_point_from_hex(tx_hash, index), ..In::new(cell) }
}

/// Builds the spec with the replayed transaction's recorded witnesses, which are often fewer than
/// its inputs, while `TxSpec` writes exactly one per input.
fn replay(context: &mut Context, spec: &TxSpec, witnesses: Vec<Bytes>) -> TransactionView {
    let tx = spec.build(context);
    tx.as_advanced_builder().set_witnesses(witnesses.into_iter().map(|w| w.pack()).collect()).build()
}
