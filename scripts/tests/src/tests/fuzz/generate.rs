use super::*;

pub(super) const SEGMENT_KINDS: [&str; 8] =
    ["deposit", "mint", "withdraw", "claim", "order_mint", "order_match", "order_melt", "dao_deposit"];

// The phase-2 claim shape of `build_many_header_phase2_batch`, whose since satisfies the DAO for
// deposits committed in epoch 35 and this withdraw header.
pub(super) const CLAIM_SINCE: u64 = 0x2003e8022a0002f3;
const ORDER_DATA_LEN: usize = 89;
const MASTER_OFFSET: usize = 52;

/// One valid piece of a transaction. Indices in `pins`, `distances` and `witness_headers` are
/// segment-local and become transaction positions when the segments are rendered together.
#[derive(Default)]
struct Segment {
    inputs: Vec<In>,
    outputs: Vec<Out>,
    headers: Vec<HeaderView>,
    // (input, output): the DAO requires a withdrawal request at its deposit's index.
    pins: Vec<(usize, usize)>,
    // (from output, to output, data offset): an i32 distance written into `from`'s data.
    distances: Vec<(usize, usize, usize)>,
    // (input, header): the DAO phase-2 witness names the deposit header's dep index.
    witness_headers: Vec<(usize, HeaderView)>,
}

struct Gen<'a> {
    rng: &'a mut Rng,
    env: &'a Env,
    next_block: u64,
}

pub(super) fn generate_case(rng: &mut Rng, env: &Env) -> (TxSpec, Vec<&'static str>) {
    let next_block = 1_000 + rng.below(500);
    let mut gen = Gen { rng, env, next_block };
    let count = gen.rng.range(1, 3);
    let mut kinds = Vec::new();
    let mut segments = Vec::new();
    for _ in 0..count {
        let kind = SEGMENT_KINDS[gen.rng.index(SEGMENT_KINDS.len())];
        kinds.push(kind);
        segments.push(match kind {
            "deposit" => gen.deposit(),
            "mint" => gen.mint(),
            "withdraw" => gen.withdraw(),
            "claim" => gen.claim(),
            "order_mint" => gen.order_mint(),
            "order_match" => gen.order_match(),
            "dao_deposit" => gen.dao_deposit(),
            _ => gen.order_melt(),
        });
    }
    (gen.render(segments), kinds)
}

fn ceil_div(a: u128, b: u128) -> u128 {
    (a + b - 1) / b
}

impl Gen<'_> {
    fn out_point(&mut self) -> OutPoint {
        OutPoint::new(self.rng.hash().pack(), self.rng.below(4) as u32)
    }

    fn input(&mut self, cell: Cell) -> In {
        In { cell, out_point: self.out_point(), header: None, since: 0, witness: Bytes::new() }
    }

    fn accumulated_rate(&mut self) -> u64 {
        match self.rng.below(3) {
            0 => GENESIS_AR,
            1 => GENESIS_AR + self.rng.below(1_000),
            _ => GENESIS_AR + self.rng.below(GENESIS_AR / 2),
        }
    }

    fn header(&mut self, ar: u64) -> HeaderView {
        let number = self.next_block;
        self.next_block += 1;
        gen_header(number, ar, 35, 1000, 1000)
    }

    /// An unused deposit capacity inside the protocol bounds, biased to the bounds and the soft cap.
    fn deposit_amount(&mut self, ar: u64) -> u64 {
        let soft_cap = (u128::from(100_000 * CKB) * u128::from(ar) / u128::from(GENESIS_AR)) as u64;
        match self.rng.below(6) {
            0 => 1_000 * CKB,
            1 => 1_000_000 * CKB,
            2 => soft_cap - 1 + self.rng.below(3),
            3 => self.rng.range(1_000, 2_000) * CKB,
            _ => self.rng.range(1_000 * CKB, 1_000_000 * CKB),
        }
    }

    fn udt_cell(&self, amount: u128) -> Cell {
        let env = self.env;
        Cell { capacity: occupied_capacity(&env.user, &env.xudt, 16), lock: env.user.clone(), type_: Some(env.xudt.clone()), data: udt_data(amount).to_vec() }
    }

    fn deposit_cell(&self, lock: &Script, amount: u64, data: Vec<u8>) -> Cell {
        let env = self.env;
        Cell { capacity: deposit_capacity(lock, &env.dao, 8, amount), lock: lock.clone(), type_: Some(env.dao.clone()), data }
    }

    fn owner_cell(&self, distance: i32) -> Cell {
        let env = self.env;
        Cell {
            capacity: occupied_capacity(&env.user, &env.owned_owner, 4),
            lock: env.user.clone(),
            type_: Some(env.owned_owner.clone()),
            data: distance.to_le_bytes().to_vec(),
        }
    }

    fn master_cell(&self) -> Cell {
        let env = self.env;
        Cell { capacity: occupied_capacity(&env.user, &env.limit_order, 0), lock: env.user.clone(), type_: Some(env.limit_order.clone()), data: vec![] }
    }

    /// Splits `total` iCKB into one or two xUDT cells.
    fn udt_outputs(&mut self, total: u128) -> Vec<Out> {
        let first = if total > 1 && self.rng.chance(1, 2) { u128::from(self.rng.below(total.min(u128::from(u64::MAX)) as u64)) } else { total };
        let mut outs = vec![Out { cell: self.udt_cell(first), change: false }];
        if first != total {
            outs.push(Out { cell: self.udt_cell(total - first), change: false });
        }
        outs
    }

    // Deposit phase 1: deposits of one amount plus receipts that count them.
    fn deposit(&mut self) -> Segment {
        let env = self.env;
        let amount = self.deposit_amount(GENESIS_AR);
        let count = self.rng.range(1, 3) as u32;
        let mut s = Segment::default();
        for _ in 0..count {
            s.outputs.push(Out { cell: self.deposit_cell(&env.ickb, amount, vec![0; 8]), change: false });
        }
        let first = if count > 1 && self.rng.chance(1, 2) { self.rng.range(1, u64::from(count) - 1) as u32 } else { count };
        for quantity in [first, count - first].into_iter().filter(|q| *q > 0) {
            let receipt = Cell {
                capacity: occupied_capacity(&env.user, &env.ickb, 12),
                lock: env.user.clone(),
                type_: Some(env.ickb.clone()),
                data: receipt_data(quantity, amount).to_vec(),
            };
            s.outputs.push(Out { cell: receipt, change: false });
        }
        s
    }

    // Deposit phase 2: a receipt converts into iCKB at its block's accumulated rate.
    fn mint(&mut self) -> Segment {
        let env = self.env;
        let ar = self.accumulated_rate();
        let amount = self.deposit_amount(ar);
        let quantity = self.rng.range(1, 3) as u32;
        let header = self.header(ar);
        let receipt = Cell {
            capacity: occupied_capacity(&env.user, &env.ickb, 12),
            lock: env.user.clone(),
            type_: Some(env.ickb.clone()),
            data: receipt_data(quantity, amount).to_vec(),
        };
        let mut input = self.input(receipt);
        input.header = Some(header.clone());
        let value = u128::from(quantity) * soft_capped_ickb(amount, ar);
        Segment { inputs: vec![input], outputs: self.udt_outputs(value), headers: vec![header], ..Default::default() }
    }

    // Withdrawal phase 1: burn a pool deposit's iCKB value into an owned-owner pair.
    fn withdraw(&mut self) -> Segment {
        let env = self.env;
        let ar = self.accumulated_rate();
        let amount = self.deposit_amount(ar);
        let header = self.header(ar);
        let mut deposit = self.input(self.deposit_cell(&env.ickb, amount, vec![0; 8]));
        deposit.header = Some(header.clone());
        let value = soft_capped_ickb(amount, ar);
        let extra = if self.rng.chance(1, 2) { u128::from(self.rng.below(10 * CKB)) } else { 0 };
        let udt = self.input(self.udt_cell(value + extra));
        let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..deposit.cell.clone() };
        let mut outputs = vec![Out { cell: request, change: false }, Out { cell: self.owner_cell(0), change: false }];
        if extra > 0 {
            outputs.push(Out { cell: self.udt_cell(extra), change: false });
        }
        Segment {
            inputs: vec![deposit, udt],
            outputs,
            headers: vec![header],
            pins: vec![(0, 0)],
            distances: vec![(1, 0, 0)],
            ..Default::default()
        }
    }

    // Withdrawal phase 2: spend a withdrawal request together with its owner cell.
    fn claim(&mut self) -> Segment {
        let env = self.env;
        let tx_hash = self.rng.hash();
        let owned_index = self.rng.below(3) as u32;
        let owner_index = (owned_index + 1 + self.rng.below(2) as u32) % 4;
        let deposit_number = 1_554 + self.rng.below(40);
        let deposit_header = gen_header(deposit_number, GENESIS_AR, 35, 1000, 1000);
        let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
        let amount = self.deposit_amount(GENESIS_AR);
        let owned = In {
            cell: self.deposit_cell(&env.owned_owner, amount, deposit_number.to_le_bytes().to_vec()),
            out_point: OutPoint::new(tx_hash.pack(), owned_index),
            header: Some(withdraw_header.clone()),
            since: CLAIM_SINCE,
            witness: Bytes::new(),
        };
        let owner = In {
            cell: self.owner_cell(owned_index as i32 - owner_index as i32),
            out_point: OutPoint::new(tx_hash.pack(), owner_index),
            // Created with its request, so it sits in the same block.
            header: Some(withdraw_header.clone()),
            since: 0,
            witness: Bytes::new(),
        };
        let claimed = dao_maximum_withdraw_capacity(&owned.cell.output(), 8, GENESIS_AR, SYNTHETIC_WITHDRAW_AR);
        let output = Cell { capacity: claimed, lock: env.user.clone(), type_: None, data: vec![] };
        let mut inputs = vec![owned, owner];
        if self.rng.chance(1, 2) {
            inputs.swap(0, 1);
        }
        let owned_at = inputs.iter().position(|i| i.since == CLAIM_SINCE).expect("owned input");
        Segment {
            inputs,
            outputs: vec![Out { cell: output, change: false }],
            headers: vec![withdraw_header, deposit_header.clone()],
            witness_headers: vec![(owned_at, deposit_header)],
            ..Default::default()
        }
    }

    // A plain NervosDAO deposit under a user lock, as a wallet may add to any transaction; the
    // wrap mutation turns it into an owned-owner pair around a cell that is not a request.
    fn dao_deposit(&mut self) -> Segment {
        let env = self.env;
        let amount = self.deposit_amount(GENESIS_AR);
        Segment { outputs: vec![Out { cell: self.deposit_cell(&env.user, amount, vec![0; 8]), change: false }], ..Default::default() }
    }

    /// The order's UDT type, ratios and minimum match log, valid for the script.
    fn order_info(&mut self) -> (Script, (u64, u64), (u64, u64), u8) {
        let udt = if self.rng.chance(1, 2) { self.env.xudt.clone() } else { self.env.helper.clone() };
        let ratio = |rng: &mut Rng| (rng.range(1, 100), rng.range(1, 100));
        let (c2u, u2c) = match self.rng.below(3) {
            0 => (ratio(self.rng), (0, 0)),
            1 => ((0, 0), ratio(self.rng)),
            _ => {
                let (a, b) = ratio(self.rng);
                let c = self.rng.range(1, 100);
                // Concave: a * d >= b * c, so a round trip never loses the maker value.
                let d = ceil_div(u128::from(b) * u128::from(c), u128::from(a)) as u64 + self.rng.below(3);
                ((a, b), (c, d))
            }
        };
        // Up to 2^36 shannons, which still fits the smallest order (1,000 CKB unused).
        (udt, c2u, u2c, self.rng.below(37) as u8)
    }

    fn order_cell(&self, udt_type: &Script, capacity: u64, data: Vec<u8>) -> Cell {
        Cell { capacity, lock: self.env.limit_order.clone(), type_: Some(udt_type.clone()), data }
    }

    /// A live order input, either still in its mint state or already matched, and its master.
    fn order_input(&mut self, udt_type: &Script, info: ((u64, u64), (u64, u64), u8), udt: u128, capacity: u64) -> (In, OutPoint) {
        let (c2u, u2c, log) = info;
        if self.rng.chance(1, 2) {
            let tx_hash = self.rng.hash();
            let order_index = self.rng.below(3) as u32;
            let master_index = (order_index + 1 + self.rng.below(2) as u32) % 4;
            let distance = (master_index as i32 - order_index as i32).to_le_bytes();
            let data = order_data_custom(udt, 0, [0; 32], distance, c2u, u2c, log).to_vec();
            let input = In {
                cell: self.order_cell(udt_type, capacity, data),
                out_point: OutPoint::new(tx_hash.pack(), order_index),
                header: None,
                since: 0,
                witness: Bytes::new(),
            };
            (input, OutPoint::new(tx_hash.pack(), master_index))
        } else {
            let master = self.out_point();
            let data = order_data_match(udt, &master, (0, 0)).to_vec();
            let mut data = data;
            write_order_info(&mut data, c2u, u2c, log);
            (self.input(self.order_cell(udt_type, capacity, data)), master)
        }
    }

    fn order_mint(&mut self) -> Segment {
        let (udt_type, c2u, u2c, log) = self.order_info();
        let is_ickb = udt_type.calc_script_hash() == self.env.xudt.calc_script_hash();
        let udt = if is_ickb && self.rng.chance(1, 2) { 0 } else { u128::from(self.rng.below(1_000_000_000_000)) };
        let capacity = deposit_capacity(&self.env.limit_order, &udt_type, ORDER_DATA_LEN, self.rng.below(2_000) * CKB);
        let data = order_data_custom(udt, 0, [0; 32], [0; 4], c2u, u2c, log).to_vec();
        let mut s = Segment {
            outputs: vec![
                Out { cell: self.order_cell(&udt_type, capacity, data), change: false },
                Out { cell: self.master_cell(), change: false },
            ],
            distances: vec![(0, 1, MASTER_OFFSET)],
            ..Default::default()
        };
        if is_ickb && udt > 0 {
            s.inputs.push(self.input(self.udt_cell(udt)));
        }
        s
    }

    fn order_match(&mut self) -> Segment {
        let (udt_type, c2u, u2c, log) = self.order_info();
        let is_ickb = udt_type.calc_script_hash() == self.env.xudt.calc_script_hash();
        let udt = u128::from(self.rng.range(1_000_000_000, 10_000_000_000_000));
        let unused = self.rng.range(1_000, 100_000) * CKB;
        let capacity = deposit_capacity(&self.env.limit_order, &udt_type, ORDER_DATA_LEN, unused);
        let (input, master) = self.order_input(&udt_type, (c2u, u2c, log), udt, capacity);
        let min_match = 1u128 << log;
        let ckb_to_udt = match (c2u != (0, 0), u2c != (0, 0)) {
            (true, false) => true,
            (false, true) => false,
            _ => self.rng.chance(1, 2),
        };
        let mut s = Segment::default();
        let (out_capacity, out_udt) = if ckb_to_udt {
            let (a, b) = (u128::from(c2u.0), u128::from(c2u.1));
            // Full fill, a fill exactly at the minimum (where one shannon less fails), or any fill between.
            let taken = match self.rng.below(4) {
                0 => u128::from(unused),
                1 => min_match,
                _ => u128::from(self.rng.range(min_match as u64, unused)),
            };
            let paid = ceil_div(taken * a, b) + u128::from(self.rng.below(3));
            if is_ickb {
                s.inputs.push(self.input(self.udt_cell(paid)));
            }
            (capacity - taken as u64, udt + paid)
        } else {
            let (c, d) = (u128::from(u2c.0), u128::from(u2c.1));
            let min_udt = ceil_div(min_match * c, d).min(udt);
            let taken = match self.rng.below(4) {
                0 => udt,
                1 => min_udt,
                _ => u128::from(self.rng.range(min_udt as u64, udt as u64)),
            };
            let paid = ceil_div(taken * d, c) + u128::from(self.rng.below(3));
            if is_ickb {
                s.outputs.push(Out { cell: self.udt_cell(taken), change: false });
            }
            (capacity + paid as u64, udt - taken)
        };
        let mut data = order_data_match(out_udt, &master, (0, 0)).to_vec();
        write_order_info(&mut data, c2u, u2c, log);
        s.inputs.push(input);
        s.outputs.push(Out { cell: self.order_cell(&udt_type, out_capacity, data), change: false });
        s
    }

    fn order_melt(&mut self) -> Segment {
        let (udt_type, c2u, u2c, log) = self.order_info();
        let is_ickb = udt_type.calc_script_hash() == self.env.xudt.calc_script_hash();
        let udt = u128::from(self.rng.below(1_000_000_000_000));
        let capacity = deposit_capacity(&self.env.limit_order, &udt_type, ORDER_DATA_LEN, self.rng.below(2_000) * CKB);
        let (order, master) = self.order_input(&udt_type, (c2u, u2c, log), udt, capacity);
        let master = In { cell: self.master_cell(), out_point: master, header: None, since: 0, witness: Bytes::new() };
        let mut s = Segment { inputs: vec![order, master], ..Default::default() };
        if is_ickb && udt > 0 {
            s.outputs.push(Out { cell: self.udt_cell(udt), change: false });
        }
        s
    }

    /// Joins the segments into one transaction in a random valid order, funded by one input
    /// whose change output absorbs every capacity difference.
    fn render(&mut self, segments: Vec<Segment>) -> TxSpec {
        let env = self.env;
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut headers: Vec<HeaderView> = Vec::new();
        let (mut pins, mut distances, mut witness_headers) = (Vec::new(), Vec::new(), Vec::new());
        for s in segments {
            let (i0, o0) = (inputs.len(), outputs.len());
            pins.extend(s.pins.iter().map(|(i, o)| (i0 + i, o0 + o)));
            distances.extend(s.distances.iter().map(|(f, t, off)| (o0 + f, o0 + t, *off)));
            witness_headers.extend(s.witness_headers.into_iter().map(|(i, h)| (i0 + i, h)));
            inputs.extend(s.inputs);
            outputs.extend(s.outputs);
            for h in s.headers {
                if !headers.iter().any(|known| known.hash() == h.hash()) {
                    headers.push(h);
                }
            }
        }
        let out_sum: u64 = outputs.iter().map(|o: &Out| o.cell.capacity).sum();
        let in_sum: u64 = inputs.iter().map(|i: &In| i.cell.capacity).sum();
        // Ample funding, so mutations that grow a cell up to the 1M CKB deposit bound stay realistic.
        let funding = Cell { capacity: out_sum.saturating_sub(in_sum) + 2_000_000 * CKB, lock: env.funding.clone(), type_: None, data: vec![] };
        inputs.push(self.input(funding));
        outputs.push(Out { cell: Cell { capacity: 0, lock: env.funding.clone(), type_: None, data: vec![] }, change: true });

        let mut order: Vec<usize> = (0..inputs.len()).collect();
        self.rng.shuffle(&mut order);
        let mut position = vec![0; inputs.len()];
        for (p, i) in order.iter().enumerate() {
            position[*i] = p;
        }
        let pinned: BTreeMap<usize, usize> = pins.iter().map(|(i, o)| (*o, position[*i])).collect();
        let mut free: Vec<usize> = (0..outputs.len()).filter(|o| !pinned.contains_key(o)).collect();
        self.rng.shuffle(&mut free);
        let slots = outputs.len().max(pinned.values().map(|p| p + 1).max().unwrap_or(0));
        let mut out_position = vec![usize::MAX; outputs.len()];
        let mut placed = Vec::with_capacity(slots);
        for p in 0..slots {
            let source = pinned.iter().find(|(_, at)| **at == p).map(|(o, _)| *o).or_else(|| free.pop());
            match source {
                Some(o) => {
                    out_position[o] = p;
                    placed.push(outputs[o].clone());
                }
                // A plain filler output keeps a pinned request at its deposit's index.
                None => placed.push(Out {
                    cell: Cell { capacity: plain_capacity(&env.user), lock: env.user.clone(), type_: None, data: vec![] },
                    change: false,
                }),
            }
        }
        for (from, to, offset) in distances {
            let distance = out_position[to] as i32 - out_position[from] as i32;
            placed[out_position[from]].cell.data[offset..offset + 4].copy_from_slice(&distance.to_le_bytes());
        }
        self.rng.shuffle(&mut headers);
        for (i, h) in witness_headers {
            let index = headers.iter().position(|known| known.hash() == h.hash()).expect("witness header") as u64;
            inputs[i].witness = header_dep_index_witness(index);
        }
        let inputs = order.iter().map(|i| inputs[*i].clone()).collect();
        TxSpec { inputs, outputs: placed, header_deps: headers }
    }
}

/// The occupied capacity of a plain cell under this lock.
fn plain_capacity(lock: &Script) -> u64 {
    Cell { capacity: 0, lock: lock.clone(), type_: None, data: vec![] }.occupied()
}

/// Writes ratios and minimum match log into an encoded order (offsets of `order_data_custom`).
fn write_order_info(data: &mut [u8], c2u: (u64, u64), u2c: (u64, u64), log: u8) {
    data[56..64].copy_from_slice(&c2u.0.to_le_bytes());
    data[64..72].copy_from_slice(&c2u.1.to_le_bytes());
    data[72..80].copy_from_slice(&u2c.0.to_le_bytes());
    data[80..88].copy_from_slice(&u2c.1.to_le_bytes());
    data[88] = log;
}
