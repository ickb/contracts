use super::*;
use primitive_types::U256;

// Invariants from the scripts' specification, re-derived from the transaction and its resolved
// cells rather than from script code paths. Each check applies only when the owning script
// executes: outputs alone do not run their locks, so the documented creation-time blind spots
// (LO-01 phantom orders, receiptless deposits, lock-only owned-owner look-alikes) stay out of
// scope, and recipients are never checked (the weak-lock boundary).

struct C {
    out: CellOutput,
    data: Vec<u8>,
    lock: Byte32,
    type_: Option<Byte32>,
    // Inputs only: the out point and the accumulated rate of its block, when that header is a dep.
    out_point: Option<OutPoint>,
    ar: Option<u64>,
}

impl C {
    fn unused(&self) -> u64 {
        let capacity: u64 = self.out.capacity().unpack();
        let occupied = self
            .out
            .occupied_capacity(ckb_testtool::ckb_types::core::Capacity::bytes(self.data.len()).expect("data length"))
            .expect("occupied")
            .as_u64();
        capacity - occupied
    }

    fn capacity(&self) -> u128 {
        let capacity: u64 = self.out.capacity().unpack();
        u128::from(capacity)
    }

    fn is_type(&self, h: &Byte32) -> bool {
        self.type_.as_ref() == Some(h)
    }

    fn udt(&self) -> Option<u128> {
        (self.data.len() >= 16).then(|| u128::from_le_bytes(self.data[..16].try_into().expect("16 bytes")))
    }
}

struct Hashes {
    ickb: Byte32,
    dao: Byte32,
    xudt: Byte32,
    owned_owner: Byte32,
    limit_order: Byte32,
}

pub(super) fn check_invariants(env: &Env, context: &Context, tx: &TransactionView) -> Vec<String> {
    let h = Hashes {
        ickb: env.ickb.calc_script_hash(),
        dao: env.dao.calc_script_hash(),
        xudt: env.xudt.calc_script_hash(),
        owned_owner: env.owned_owner.calc_script_hash(),
        limit_order: env.limit_order.calc_script_hash(),
    };
    let deps: Vec<Byte32> = tx.header_deps().into_iter().collect();
    let inputs: Vec<C> = tx
        .inputs()
        .into_iter()
        .map(|input| {
            let op = input.previous_output();
            let (out, data) = context.get_cell(&op).expect("input cell");
            let ar = context
                .transaction_infos
                .get(&op)
                .map(|info| info.block_hash.clone())
                .filter(|hash| deps.contains(hash))
                .and_then(|hash| context.headers.get(&hash))
                .map(|header| extract_dao_data(header.dao()).0);
            C { lock: out.lock().calc_script_hash(), type_: out.type_().to_opt().map(|t| t.calc_script_hash()), out, data: data.to_vec(), out_point: Some(op), ar }
        })
        .collect();
    let outputs: Vec<C> = tx
        .outputs()
        .into_iter()
        .zip(tx.outputs_data())
        .map(|(out, data)| C {
            lock: out.lock().calc_script_hash(),
            type_: out.type_().to_opt().map(|t| t.calc_script_hash()),
            out,
            data: data.raw_data().to_vec(),
            out_point: None,
            ar: None,
        })
        .collect();

    let runs = |s: &Byte32| inputs.iter().any(|c| &c.lock == s) || inputs.iter().chain(&outputs).any(|c| c.is_type(s));
    let mut v = Vec::new();
    ickb_conservation(&h, runs(&h.ickb), &inputs, &outputs, &mut v);
    if runs(&h.ickb) {
        receipts_match_deposits(&h, &outputs, &mut v);
    }
    if runs(&h.owned_owner) {
        owned_owner_pairs(&h, &inputs, "input", &mut v);
        owned_owner_pairs(&h, &outputs, "output", &mut v);
    }
    if runs(&h.limit_order) {
        limit_orders(&h, &inputs, &outputs, &mut v);
    }
    v
}

fn is_pool_deposit(h: &Hashes, c: &C) -> bool {
    c.lock == h.ickb && c.is_type(&h.dao) && c.data == [0u8; 8]
}

fn udt_total(h: &Hashes, cells: &[C], side: &str, v: &mut Vec<String>) -> u128 {
    let mut total = 0u128;
    for (i, c) in cells.iter().enumerate().filter(|(_, c)| c.is_type(&h.xudt)) {
        match c.udt() {
            Some(amount) => total = total.saturating_add(amount),
            None => v.push(format!("{side} {i}: iCKB xUDT cell with {} data bytes accepted", c.data.len())),
        }
    }
    total
}

// iCKB supply moves only by redeemed receipts and burned pool deposits.
fn ickb_conservation(h: &Hashes, runs: bool, inputs: &[C], outputs: &[C], v: &mut Vec<String>) {
    let in_udt = udt_total(h, inputs, "input", v);
    let out_udt = udt_total(h, outputs, "output", v);
    if !runs {
        if out_udt > in_udt {
            v.push(format!("iCKB supply grew from {in_udt} to {out_udt} without ickb_logic running"));
        }
        return;
    }
    let mut receipts = 0u128;
    let mut deposits = 0u128;
    for (i, c) in inputs.iter().enumerate() {
        if c.is_type(&h.ickb) {
            if c.data.len() < 12 {
                v.push(format!("input {i}: receipt with {} data bytes accepted", c.data.len()));
                continue;
            }
            let quantity = u32::from_le_bytes(c.data[..4].try_into().expect("4 bytes"));
            let amount = u64::from_le_bytes(c.data[4..12].try_into().expect("8 bytes"));
            match c.ar {
                Some(ar) => receipts += u128::from(quantity) * soft_capped_ickb(amount, ar),
                None => v.push(format!("input {i}: receipt redeemed without its block header in header deps")),
            }
        }
        if c.lock == h.ickb {
            if !is_pool_deposit(h, c) {
                v.push(format!("input {i}: ickb_logic-locked cell that is not a pool deposit was spent"));
                continue;
            }
            match c.ar {
                Some(ar) => deposits += soft_capped_ickb(c.unused(), ar),
                None => v.push(format!("input {i}: pool deposit spent without its block header in header deps")),
            }
        }
    }
    // 256-bit sums: mutated amounts can reach u128::MAX.
    if U256::from(in_udt) + U256::from(receipts) != U256::from(out_udt) + U256::from(deposits) {
        v.push(format!(
            "iCKB not conserved: input xUDT {in_udt} + receipts {receipts} != output xUDT {out_udt} + spent deposits {deposits}"
        ));
    }
}

// Every receipt created counts pool deposits created in the same transaction, one for one.
fn receipts_match_deposits(h: &Hashes, outputs: &[C], v: &mut Vec<String>) {
    let mut buckets: BTreeMap<u64, (u128, u128)> = BTreeMap::new();
    for (i, c) in outputs.iter().enumerate() {
        if is_pool_deposit(h, c) {
            let amount = c.unused();
            if !(1_000 * CKB..=1_000_000 * CKB).contains(&amount) {
                v.push(format!("output {i}: deposit of {amount} shannons outside 1,000..1,000,000 CKB created"));
            }
            buckets.entry(amount).or_default().0 += 1;
        } else if c.lock == h.ickb {
            v.push(format!("output {i}: ickb_logic-locked cell that is not a pool deposit created"));
        }
        if c.is_type(&h.ickb) {
            if c.data.len() < 12 {
                v.push(format!("output {i}: receipt with {} data bytes created", c.data.len()));
                continue;
            }
            let quantity = u32::from_le_bytes(c.data[..4].try_into().expect("4 bytes"));
            let amount = u64::from_le_bytes(c.data[4..12].try_into().expect("8 bytes"));
            if quantity == 0 {
                v.push(format!("output {i}: empty receipt created"));
            }
            buckets.entry(amount).or_default().1 += u128::from(quantity);
        }
    }
    for (amount, (deposited, receipted)) in buckets {
        if deposited != receipted {
            v.push(format!("{deposited} deposits of {amount} shannons created against receipts for {receipted}"));
        }
    }
}

type Point = (Option<Vec<u8>>, i64);

fn point_of(c: &C, index: usize, offset: i64) -> Point {
    match &c.out_point {
        Some(op) => {
            let i: u32 = op.index().unpack();
            (Some(op.tx_hash().as_slice().to_vec()), i64::from(i) + offset)
        }
        None => (None, index as i64 + offset),
    }
}

// Within one side, each owned withdrawal request pairs with exactly one owner cell.
fn owned_owner_pairs(h: &Hashes, cells: &[C], side: &str, v: &mut Vec<String>) {
    let mut pairs: BTreeMap<Point, (u32, u32)> = BTreeMap::new();
    for (i, c) in cells.iter().enumerate() {
        let (owned, owner) = (c.lock == h.owned_owner, c.is_type(&h.owned_owner));
        if owned && owner {
            v.push(format!("{side} {i}: owned_owner as both lock and type"));
        } else if owned {
            if !(c.is_type(&h.dao) && c.data.len() == 8 && c.data != [0u8; 8]) {
                v.push(format!("{side} {i}: owned cell that is not a DAO withdrawal request"));
            }
            pairs.entry(point_of(c, i, 0)).or_default().0 += 1;
        } else if owner {
            if c.data.len() < 4 {
                v.push(format!("{side} {i}: owner cell with {} data bytes", c.data.len()));
                continue;
            }
            let distance = i32::from_le_bytes(c.data[..4].try_into().expect("4 bytes"));
            pairs.entry(point_of(c, i, i64::from(distance))).or_default().1 += 1;
        }
    }
    for (point, (owned, owner)) in pairs {
        if (owned, owner) != (1, 1) {
            v.push(format!("{side}s: {owned} owned and {owner} owner cells share {point:?}"));
        }
    }
}

#[derive(PartialEq, Debug)]
struct Info {
    udt_type: Option<Byte32>,
    c2u: Option<(u128, u128)>,
    u2c: Option<(u128, u128)>,
    min_match: u128,
}

struct Order {
    master: Point,
    is_mint: bool,
    ckb: u128,
    unused: u128,
    udt: u128,
    info: Info,
}

/// The order encoding as the whitepaper specifies it; None when the script must reject it.
fn parse_order(c: &C, index: usize) -> Option<Order> {
    let d = &c.data;
    if d.len() != 89 {
        return None;
    }
    let u64_at = |o: usize| u64::from_le_bytes(d[o..o + 8].try_into().expect("8 bytes"));
    let ratio = |o: usize| match (u64_at(o), u64_at(o + 8)) {
        (0, 0) => Some(None),
        (0, _) | (_, 0) => None,
        (a, b) => Some(Some((u128::from(a), u128::from(b)))),
    };
    let (c2u, u2c) = (ratio(56)?, ratio(72)?);
    match (c2u, u2c) {
        (None, None) => return None,
        (Some((a, b)), Some((c2, d2))) if a * d2 < b * c2 => return None,
        _ => (),
    }
    if d[88] > 64 {
        return None;
    }
    let action = u32::from_le_bytes(d[16..20].try_into().expect("4 bytes"));
    let index_bytes: [u8; 4] = d[52..56].try_into().expect("4 bytes");
    let master = match action {
        0 if d[20..52] == [0u8; 32] => point_of(c, index, i64::from(i32::from_le_bytes(index_bytes))),
        1 => (Some(d[20..52].to_vec()), i64::from(u32::from_le_bytes(index_bytes))),
        _ => return None,
    };
    c.type_.as_ref()?;
    Some(Order {
        master,
        is_mint: action == 0,
        ckb: c.capacity(),
        unused: u128::from(c.unused()),
        udt: u128::from_le_bytes(d[..16].try_into().expect("16 bytes")),
        info: Info { udt_type: c.type_.clone(), c2u, u2c, min_match: 1u128 << d[88] },
    })
}

#[derive(Default)]
struct Slot {
    in_orders: Vec<(usize, Order)>,
    in_masters: u32,
    out_orders: Vec<(usize, Order)>,
    out_masters: u32,
}

// A matched order keeps its master and info and never loses value at its own ratio; spending
// an order without its master leaves exactly one continuation; masters move only with orders.
fn limit_orders(h: &Hashes, inputs: &[C], outputs: &[C], v: &mut Vec<String>) {
    let mut slots: BTreeMap<Point, Slot> = BTreeMap::new();
    for (side, cells) in [("input", inputs), ("output", outputs)] {
        for (i, c) in cells.iter().enumerate() {
            let (order, master) = (c.lock == h.limit_order, c.is_type(&h.limit_order));
            if order && master {
                v.push(format!("{side} {i}: limit_order as both lock and type"));
            } else if order {
                let Some(o) = parse_order(c, i) else {
                    v.push(format!("{side} {i}: limit order with invalid data"));
                    continue;
                };
                let slot = slots.entry(o.master.clone()).or_default();
                if side == "input" { &mut slot.in_orders } else { &mut slot.out_orders }.push((i, o));
            } else if master {
                let slot = slots.entry(point_of(c, i, 0)).or_default();
                if side == "input" { slot.in_masters += 1 } else { slot.out_masters += 1 }
            }
        }
    }
    for (point, s) in slots {
        if s.in_orders.len() > 1 || s.out_orders.len() > 1 || s.in_masters > 1 || s.out_masters > 1 {
            v.push(format!("master {point:?} shared by several orders or masters"));
            continue;
        }
        match (s.in_orders.first(), s.in_masters, s.out_orders.first(), s.out_masters) {
            (Some(_), 1, None, 0) => (),
            (None, 0, Some((_, o)), 1) if o.is_mint => (),
            (Some((i, a)), 0, Some((j, b)), 0) => continuation(*i, a, *j, b, v),
            _ => v.push(format!(
                "master {point:?}: {} input orders, {} input masters, {} output orders, {} output masters",
                s.in_orders.len(),
                s.in_masters,
                s.out_orders.len(),
                s.out_masters
            )),
        }
    }
}

fn continuation(i: usize, a: &Order, j: usize, b: &Order, v: &mut Vec<String>) {
    let what = format!("input order {i} -> output order {j}");
    if a.info != b.info {
        v.push(format!("{what}: info changed {:?} -> {:?}", a.info, b.info));
        return;
    }
    if b.ckb < a.ckb && b.udt < a.udt {
        v.push(format!("{what}: lost both CKB {} -> {} and UDT {} -> {}", a.ckb, b.ckb, a.udt, b.udt));
    } else if b.ckb < a.ckb {
        let Some((ckb_mul, udt_mul)) = a.info.c2u else {
            return v.push(format!("{what}: gave CKB without a CKB->UDT ratio"));
        };
        if value(b, ckb_mul, udt_mul) < value(a, ckb_mul, udt_mul) {
            v.push(format!("{what}: CKB->UDT value decreased (ckb {} -> {}, udt {} -> {})", a.ckb, b.ckb, a.udt, b.udt));
        }
        if a.unused == 0 || (b.unused != 0 && a.ckb - b.ckb < a.info.min_match) {
            v.push(format!("{what}: CKB->UDT fill of {} below minimum {} or on a fulfilled order", a.ckb - b.ckb, a.info.min_match));
        }
    } else if b.udt < a.udt {
        let Some((ckb_mul, udt_mul)) = a.info.u2c else {
            return v.push(format!("{what}: gave UDT without a UDT->CKB ratio"));
        };
        if value(b, ckb_mul, udt_mul) < value(a, ckb_mul, udt_mul) {
            v.push(format!("{what}: UDT->CKB value decreased (ckb {} -> {}, udt {} -> {})", a.ckb, b.ckb, a.udt, b.udt));
        }
        if b.udt != 0 && U256::from(a.udt - b.udt) * U256::from(udt_mul) < U256::from(a.info.min_match) * U256::from(ckb_mul) {
            v.push(format!("{what}: UDT->CKB fill of {} below minimum", a.udt - b.udt));
        }
    }
}

// The script compares in 256 bits; amounts and ratios here can reach u64 and u128 limits.
fn value(o: &Order, ckb_mul: u128, udt_mul: u128) -> U256 {
    U256::from(o.ckb) * U256::from(ckb_mul) + U256::from(o.udt) * U256::from(udt_mul)
}

// Positive control: every clean run above is only as strong as the oracle's ability to report a
// break. Each transaction here breaks one invariant and is never verified, only checked.
#[test]
fn oracle_flags_one_break_per_invariant() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let plain = |lock: &Script, capacity: u64| Cell { capacity, lock: lock.clone(), type_: None, data: vec![] };
    let input = |n: u8, cell: Cell| In {
        cell,
        out_point: OutPoint::new([n; 32].pack(), 0),
        header: None,
        since: 0,
        witness: Bytes::new(),
    };
    let output = |cell: Cell| Out { cell, change: false };
    let udt = Cell { capacity: 200 * CKB, lock: env.user.clone(), type_: Some(env.xudt.clone()), data: udt_data(1).to_vec() };
    let deposit = Cell {
        capacity: deposit_capacity(&env.ickb, &env.dao, 8, 1_000 * CKB),
        lock: env.ickb.clone(),
        type_: Some(env.dao.clone()),
        data: vec![0; 8],
    };
    let receipt = Cell { capacity: 200 * CKB, lock: env.user.clone(), type_: Some(env.ickb.clone()), data: receipt_data(2, 1_000 * CKB).to_vec() };
    let owner = Cell { capacity: 200 * CKB, lock: env.user.clone(), type_: Some(env.owned_owner.clone()), data: 1i32.to_le_bytes().to_vec() };
    let master = OutPoint::new([9; 32].pack(), 1);
    let order = |capacity: u64| Cell {
        capacity,
        lock: env.limit_order.clone(),
        type_: Some(env.helper.clone()),
        data: order_data_match(100, &master, (1, 1)).to_vec(),
    };
    let cases = [
        ("iCKB from nothing", vec![input(1, plain(&env.user, 1_000 * CKB))], vec![output(udt)]),
        ("receipt for two deposits, one created", vec![input(2, plain(&env.user, 3_000 * CKB))], vec![output(deposit), output(receipt)]),
        ("owner without its owned cell", vec![input(3, plain(&env.user, 1_000 * CKB))], vec![output(owner)]),
        ("order continuation losing CKB", vec![input(4, order(3_000 * CKB))], vec![output(order(2_000 * CKB))]),
    ];
    for (name, inputs, outputs) in cases {
        let tx = TxSpec { inputs, outputs, header_deps: vec![] }.build(&mut context);
        assert!(!check_invariants(&env, &context, &tx).is_empty(), "oracle missed: {name}");
    }
}
