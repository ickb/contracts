use super::*;

mod crosswire_blockers;
mod crosswire_live_match;
mod crosswire_mint_creation;
mod fake_match_lineage;
mod fulfilled;
mod lock_only_shapes;
mod master_admission;
mod match_progression;
mod mint_shape_validation;
mod phantom_lineage;
mod poisoning;
mod real_lineage;
mod remint;
mod same_master;

impl Env {
    /// An order cell holding `capacity`, typed by the helper UDT.
    fn order(&self, capacity: u64, data: Bytes) -> Cell {
        Cell { capacity, lock: self.limit_order.clone(), type_: Some(self.helper.clone()), data: data.to_vec() }
    }

    /// An order cell holding `unused` CKB above its occupied capacity.
    fn order_unused(&self, unused: u64, data: Bytes) -> Cell {
        let order = self.order(0, data);
        Cell { capacity: order.occupied() + unused, ..order }
    }
}

/// A UDT->CKB match order at master index 5, with `udt` left and this minimum-match log.
fn udt_to_ckb(udt: u128, ckb_min_match_log: u8) -> Bytes {
    order_data_custom(udt, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), ckb_min_match_log)
}

/// A UDT->CKB order holding 1_500 CKB unused, continued with `paid` more CKB as its UDT goes from
/// `udt_in` to `udt_out`, paid from a 100 CKB funding input that takes the rest back.
fn udt_to_ckb_step(env: &Env, paid: u64, (udt_in, udt_out): (u128, u128), ckb_min_match_log: u8) -> TxSpec {
    TxSpec {
        inputs: vec![
            In::new(env.order_unused(1_500 * CKB, udt_to_ckb(udt_in, ckb_min_match_log))),
            In::new(env.plain(100 * CKB)),
        ],
        outputs: vec![
            env.order_unused(1_500 * CKB + paid, udt_to_ckb(udt_out, ckb_min_match_log)).into(),
            env.plain(100 * CKB - paid).into(),
        ],
        ..Default::default()
    }
}


/// The live cell at `out_point` as an input, so a lineage a fixture seeded continues on this layer.
fn live(context: &Context, out_point: OutPoint) -> In {
    let (output, data) = context.get_cell(&out_point).expect("live cell");
    let cell =
        Cell { capacity: output.capacity().unpack(), lock: output.lock(), type_: output.type_().to_opt(), data: data.to_vec() };
    In { out_point, ..In::new(cell) }
}

