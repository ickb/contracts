use super::*;

/// Applies one random mutation and returns its literal description.
pub(super) fn mutate(rng: &mut Rng, env: &Env, spec: &mut TxSpec) -> String {
    for _ in 0..8 {
        if let Some(done) = try_mutate(rng, env, spec) {
            return done;
        }
    }
    "none applicable".into()
}

#[derive(Clone, Copy)]
enum Side {
    Input,
    Output,
}

fn pick_cell(rng: &mut Rng, spec: &TxSpec) -> Option<(Side, usize)> {
    let outputs: Vec<usize> = (0..spec.outputs.len()).filter(|i| !spec.outputs[*i].change).collect();
    if rng.chance(1, 2) && !spec.inputs.is_empty() {
        Some((Side::Input, rng.index(spec.inputs.len())))
    } else if !outputs.is_empty() {
        Some((Side::Output, outputs[rng.index(outputs.len())]))
    } else {
        None
    }
}

fn cell_mut(spec: &mut TxSpec, at: (Side, usize)) -> &mut Cell {
    match at {
        (Side::Input, i) => &mut spec.inputs[i].cell,
        (Side::Output, i) => &mut spec.outputs[i].cell,
    }
}

fn name(at: (Side, usize)) -> String {
    match at {
        (Side::Input, i) => format!("input {i}"),
        (Side::Output, i) => format!("output {i}"),
    }
}

/// A new integer value near, at, or far from `value`, within `width` bytes.
fn tweak(rng: &mut Rng, value: u128, width: usize) -> u128 {
    let max = if width >= 16 { u128::MAX } else { (1u128 << (8 * width)) - 1 };
    let small = u128::from(rng.range(1, 1_000));
    let v = match rng.below(9) {
        0 => value.wrapping_add(1),
        1 => value.wrapping_sub(1),
        2 => value.wrapping_add(small),
        3 => value.wrapping_sub(small),
        4 => value.wrapping_mul(2),
        5 => value / 2,
        6 => 0,
        7 => max,
        _ => u128::from(rng.next()) | (u128::from(rng.next()) << 64),
    };
    v & max
}

/// Integer fields of the data layouts the scripts read, keyed by the cell's scripts.
fn fields(env: &Env, cell: &Cell) -> Vec<(usize, usize)> {
    let type_hash = cell.type_.as_ref().map(|t| t.calc_script_hash());
    let is_type = |s: &Script| type_hash == Some(s.calc_script_hash());
    let mut f = Vec::new();
    if cell.lock.calc_script_hash() == env.limit_order.calc_script_hash() {
        // udt, action, master tx hash (first 8 bytes), master index or distance, ratios, min match log.
        // Whole ratio pairs too, so one tweak can null a ratio (BothRatioNull, InvalidRatio).
        f.extend([(0, 16), (16, 4), (20, 8), (52, 4), (56, 8), (64, 8), (72, 8), (80, 8), (56, 16), (72, 16), (88, 1)]);
    } else if is_type(&env.xudt) {
        f.push((0, 16));
    } else if is_type(&env.ickb) {
        f.extend([(0, 4), (4, 8)]);
    } else if is_type(&env.owned_owner) {
        f.push((0, 4));
    } else if is_type(&env.dao) {
        f.push((0, 8));
    }
    f.retain(|(offset, width)| offset + width <= cell.data.len());
    f
}

fn try_mutate(rng: &mut Rng, env: &Env, spec: &mut TxSpec) -> Option<String> {
    // Arm 13 has no case of its own and falls through to the re-point arm, doubling its weight;
    // renumbering would change what every recorded seed generates.
    match rng.below(17) {
        0 => {
            let at = pick_cell(rng, spec)?;
            let c = cell_mut(spec, at);
            let before = c.capacity;
            // Unused capacity at the deposit bounds, where DepositTooSmall and DepositTooBig switch.
            let bounds = [1_000 * CKB - 1, 1_000 * CKB, 1_000_000 * CKB, 1_000_000 * CKB + 1];
            c.capacity = match rng.below(6) {
                0 => before.wrapping_add(1),
                1 => before.saturating_sub(1),
                2 => before.saturating_add(rng.range(1, 2_000) * CKB),
                3 => before.saturating_sub(rng.range(1, 2_000) * CKB),
                4 => c.occupied() + bounds[rng.index(bounds.len())],
                _ => c.occupied(),
            };
            Some(format!("{} capacity {before} -> {}", name(at), c.capacity))
        }
        1 | 2 => {
            let at = pick_cell(rng, spec)?;
            let f = fields(env, cell_mut(spec, at));
            if f.is_empty() {
                return None;
            }
            let (offset, width) = f[rng.index(f.len())];
            let c = cell_mut(spec, at);
            let mut bytes = [0u8; 16];
            bytes[..width].copy_from_slice(&c.data[offset..offset + width]);
            let before = u128::from_le_bytes(bytes);
            let after = tweak(rng, before, width);
            c.data[offset..offset + width].copy_from_slice(&after.to_le_bytes()[..width]);
            Some(format!("{} data[{offset}..{}] {before} -> {after}", name(at), offset + width))
        }
        3 => {
            let at = pick_cell(rng, spec)?;
            let c = cell_mut(spec, at);
            let before = c.data.len();
            if rng.chance(1, 2) || before == 0 {
                let extra = rng.range(1, 8) as usize;
                c.data.extend((0..extra).map(|_| rng.next() as u8));
            } else {
                c.data.truncate(rng.index(before));
            }
            Some(format!("{} data length {before} -> {}", name(at), c.data.len()))
        }
        4 => {
            let n = spec.outputs.len();
            let (a, b) = (rng.index(n), rng.index(n));
            if a == b {
                return None;
            }
            spec.outputs.swap(a, b);
            Some(format!("swap outputs {a} and {b}"))
        }
        5 => {
            let n = spec.inputs.len();
            let (a, b) = (rng.index(n), rng.index(n));
            if a == b {
                return None;
            }
            spec.inputs.swap(a, b);
            Some(format!("swap inputs {a} and {b}"))
        }
        6 => {
            let at = pick_cell(rng, spec)?;
            match at {
                (Side::Input, i) => {
                    let mut copy = spec.inputs[i].clone();
                    copy.out_point = OutPoint::new(rng.hash().pack(), 0);
                    let at = rng.index(spec.inputs.len() + 1);
                    spec.inputs.insert(at, copy);
                    Some(format!("duplicate input {i} as a fresh cell at {at}"))
                }
                (Side::Output, i) => {
                    let copy = spec.outputs[i].clone();
                    let at = rng.index(spec.outputs.len() + 1);
                    spec.outputs.insert(at, copy);
                    Some(format!("duplicate output {i} at {at}"))
                }
            }
        }
        7 => {
            let at = pick_cell(rng, spec)?;
            match at {
                (Side::Input, i) => drop(spec.inputs.remove(i)),
                (Side::Output, i) => drop(spec.outputs.remove(i)),
            }
            Some(format!("remove {}", name(at)))
        }
        8 | 9 => {
            let at = pick_cell(rng, spec)?;
            let pool = env.script_pool();
            let pick = pool[rng.index(pool.len())].clone();
            let c = cell_mut(spec, at);
            if rng.chance(1, 2) {
                c.lock = pick;
                Some(format!("{} lock -> {}", name(at), hex::encode(&pick_hash(&c.lock)[..4])))
            } else {
                c.type_ = if rng.chance(1, 6) { None } else { Some(pick) };
                Some(format!("{} type -> {:?}", name(at), c.type_.as_ref().map(|t| hex::encode(&pick_hash(t)[..4]))))
            }
        }
        10 => {
            // Move a cell to another block: anyone can choose which block their cell lands in.
            let with_header: Vec<usize> = (0..spec.inputs.len()).filter(|i| spec.inputs[*i].header.is_some()).collect();
            if with_header.is_empty() {
                return None;
            }
            let i = with_header[rng.index(with_header.len())];
            let old = spec.inputs[i].header.clone().expect("header");
            let ar = match rng.below(3) {
                0 => extract_dao_data(old.dao()).0 + 1,
                1 => extract_dao_data(old.dao()).0.saturating_sub(1).max(GENESIS_AR),
                _ => GENESIS_AR + rng.below(GENESIS_AR),
            };
            let zero = ckb_testtool::ckb_types::core::Capacity::zero();
            let new = old.as_advanced_builder().dao(ckb_dao_utils::pack_dao_data(ar, zero, zero, zero)).build();
            spec.inputs[i].header = Some(new.clone());
            if rng.chance(3, 4) {
                for h in spec.header_deps.iter_mut().filter(|h| h.hash() == old.hash()) {
                    *h = new.clone();
                }
                if !spec.header_deps.iter().any(|h| h.hash() == new.hash()) {
                    spec.header_deps.push(new);
                }
            }
            Some(format!("input {i} header ar -> {ar}"))
        }
        11 => {
            if spec.header_deps.is_empty() {
                return None;
            }
            let i = rng.index(spec.header_deps.len());
            if rng.chance(1, 2) {
                spec.header_deps.remove(i);
                Some(format!("remove header dep {i}"))
            } else {
                let j = rng.index(spec.header_deps.len());
                spec.header_deps.swap(i, j);
                Some(format!("swap header deps {i} and {j}"))
            }
        }
        12 => {
            // Shift iCKB between two xUDT outputs, keeping the total.
            let udt: Vec<usize> = (0..spec.outputs.len())
                .filter(|i| {
                    let c = &spec.outputs[*i].cell;
                    c.data.len() >= 16 && c.type_.as_ref().map(|t| t.calc_script_hash()) == Some(env.xudt.calc_script_hash())
                })
                .collect();
            if udt.len() < 2 {
                return None;
            }
            let (a, b) = (udt[rng.index(udt.len())], udt[rng.index(udt.len())]);
            if a == b {
                return None;
            }
            let amount = |c: &Cell| u128::from_le_bytes(c.data[..16].try_into().expect("udt"));
            let from = amount(&spec.outputs[a].cell);
            let moved = u128::from(rng.next()).min(from);
            let to = amount(&spec.outputs[b].cell).checked_add(moved)?;
            spec.outputs[a].cell.data[..16].copy_from_slice(&(from - moved).to_le_bytes());
            spec.outputs[b].cell.data[..16].copy_from_slice(&to.to_le_bytes());
            Some(format!("move {moved} iCKB from output {a} to output {b}"))
        }
        14 => {
            // Wrap a DAO output into an owned-owner pair, as phase 1 does for real requests.
            let dao = env.dao.calc_script_hash();
            let cells: Vec<usize> = (0..spec.outputs.len())
                .filter(|i| spec.outputs[*i].cell.type_.as_ref().map(|t| t.calc_script_hash()) == Some(dao.clone()))
                .collect();
            if cells.is_empty() {
                return None;
            }
            let target = cells[rng.index(cells.len())];
            spec.outputs[target].cell.lock = env.owned_owner.clone();
            let at = rng.index(spec.outputs.len() + 1);
            let target_now = if at <= target { target + 1 } else { target };
            let owner = Cell {
                capacity: occupied_capacity(&env.user, &env.owned_owner, 4),
                lock: env.user.clone(),
                type_: Some(env.owned_owner.clone()),
                data: (target_now as i32 - at as i32).to_le_bytes().to_vec(),
            };
            spec.outputs.insert(at, Out { cell: owner, change: false });
            Some(format!("wrap output {target} into an owned-owner pair with an owner at {at}"))
        }
        15 => {
            // Move every created deposit of one amount and the receipts counting them to a new
            // amount together, so the bucket stays matched and only the bounds decide.
            let (ickb, dao) = (env.ickb.calc_script_hash(), env.dao.calc_script_hash());
            let is_deposit = |c: &Cell| {
                c.lock.calc_script_hash() == ickb
                    && c.type_.as_ref().map(|t| t.calc_script_hash()) == Some(dao.clone())
                    && c.data == [0u8; 8]
            };
            let deposits: Vec<usize> = (0..spec.outputs.len()).filter(|i| is_deposit(&spec.outputs[*i].cell)).collect();
            if deposits.is_empty() {
                return None;
            }
            let pick = &spec.outputs[deposits[rng.index(deposits.len())]].cell;
            let from = pick.capacity.checked_sub(pick.occupied())?;
            let to = match rng.below(5) {
                0 => 1_000 * CKB - 1,
                1 => 1_000_000 * CKB + 1,
                2 => from + 1,
                3 => from.saturating_sub(1),
                _ => rng.range(1, 2_000_000) * CKB,
            };
            for o in spec.outputs.iter_mut() {
                let c = &mut o.cell;
                if is_deposit(c) && c.capacity.checked_sub(c.occupied()) == Some(from) {
                    c.capacity = c.occupied().saturating_add(to);
                } else if c.type_.as_ref().map(|t| t.calc_script_hash()) == Some(ickb.clone())
                    && c.data.len() >= 12
                    && c.data[4..12] == from.to_le_bytes()
                {
                    c.data[4..12].copy_from_slice(&to.to_le_bytes());
                }
            }
            Some(format!("move deposits and receipts of {from} shannons to {to}"))
        }
        16 => {
            // Move a limit order's output one unit off its match: where the minimum match and
            // value checks switch.
            let orders: Vec<usize> = (0..spec.outputs.len())
                .filter(|i| {
                    let c = &spec.outputs[*i].cell;
                    c.lock.calc_script_hash() == env.limit_order.calc_script_hash() && c.data.len() >= 16
                })
                .collect();
            if orders.is_empty() {
                return None;
            }
            let i = orders[rng.index(orders.len())];
            let c = &mut spec.outputs[i].cell;
            let up = rng.chance(1, 2);
            if rng.chance(1, 2) {
                c.capacity = if up { c.capacity.saturating_add(1) } else { c.capacity.saturating_sub(1) };
                Some(format!("output order {i} capacity {}1", if up { "+" } else { "-" }))
            } else {
                let udt = u128::from_le_bytes(c.data[..16].try_into().expect("udt"));
                let udt = if up { udt.wrapping_add(1) } else { udt.wrapping_sub(1) };
                c.data[..16].copy_from_slice(&udt.to_le_bytes());
                Some(format!("output order {i} udt {}1", if up { "+" } else { "-" }))
            }
        }
        _ => {
            // Re-point an input: its out point decides owner, master and receipt bindings.
            if spec.inputs.is_empty() {
                return None;
            }
            let i = rng.index(spec.inputs.len());
            let op = spec.inputs[i].out_point.clone();
            let index: u32 = op.index().unpack();
            let index = match rng.below(3) {
                0 => index.wrapping_add(1),
                1 => index.wrapping_sub(1),
                _ => rng.below(8) as u32,
            };
            let j = rng.index(spec.inputs.len());
            let tx_hash = if rng.chance(1, 2) { spec.inputs[j].out_point.tx_hash() } else { op.tx_hash() };
            let moved = format!("input {i} out point -> {}:{index}", hex::encode(&tx_hash.as_slice()[..4]));
            spec.inputs[i].out_point = OutPoint::new(tx_hash, index);
            Some(moved)
        }
    }
}

fn pick_hash(s: &Script) -> Vec<u8> {
    s.calc_script_hash().as_slice().to_vec()
}
