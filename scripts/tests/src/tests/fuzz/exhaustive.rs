use super::*;

// Bounded exhaustive search (small-scope hypothesis): every transaction of up to MAX_INPUTS
// inputs and MAX_OUTPUTS outputs, MAX_CELLS in all, over a fixed alphabet of cell archetypes, in every order, plus
// one funding input and one change output. The archetypes sit at the scripts' decision points
// and their amounts derive from one deposit block, so balanced variants exist next to their
// off-by-one neighbours. Each realistic transaction is verified, and each accepted one goes
// through the fuzz oracle. The full search is `#[ignore]`d: its 37,797,412 cases took 3.7 hours
// in --release as 20 `ICKB_EXHAUSTIVE_SHARD=i/n` processes on a 12-core laptop.
//
// Symmetry choices:
// - The funding input and change output always come last. They are plain cells no script reads,
//   so moving them only shifts indices, which the archetypes already vary through their own
//   positions and distances.
// - An input's out point is (its creating transaction, its input position): two archetypes from
//   one transaction sit at different indices, and owner distances, mint-state order distances and
//   the master a matched order names resolve against input positions, as on chain.

const MAX_INPUTS: usize = 3;
const MAX_OUTPUTS: usize = 3;
// Three inputs reach a DAO claim or an order melt combined with another flow; capping the total
// keeps the 3-in-3-out block, which would quadruple the runtime, out.
const MAX_CELLS: usize = 5;

// Not a round multiple of the genesis rate, so the iCKB conversion floors.
const DEPOSIT_AR: u64 = GENESIS_AR + GENESIS_AR / 3;
const MIN_DEPOSIT: u64 = 1_000 * CKB;
const MAX_DEPOSIT: u64 = 1_000_000 * CKB;
const SOFT_CAP: u128 = 100_000 * CKB as u128;
// Same info on every order: both directions at 1:1 (concave), a 2^30 shannon minimum match.
const RATIO: (u64, u64) = (1, 1);
const MIN_MATCH_LOG: u8 = 30;
const OTHER_MIN_MATCH_LOG: u8 = 31;

// Creating transactions; an input's tx hash is the hash of its group tag.
const POOL_TX: u8 = 1; // deposits and receipts, committed in the deposit block
const UDT_TX: u8 = 2;
const WITHDRAW_TX: u8 = 3; // withdrawal request and owner cell, committed in the withdrawal block
const MINT_TX: u8 = 4; // order mint: mint-state orders and the master
const MATCH_TX: u8 = 5; // a later match: matched orders naming the master at (MINT_TX, 0)
const SIBLING_TX: u8 = 6;
const FUNDING_TX: u8 = 7;

struct InArch {
    name: &'static str,
    cell: Cell,
    group: u8,
    // The block the cell was committed in.
    block: Option<HeaderView>,
    // Header deps a valid spend needs; look-alike "nohdr" archetypes omit them.
    deps: Vec<HeaderView>,
    since: u64,
    // DAO phase 2: the witness names this deposit header's dep index.
    claim: Option<HeaderView>,
}

struct Alphabet {
    inputs: Vec<InArch>,
    outputs: Vec<(&'static str, Cell)>,
    funding: Cell,
}

fn group_hash(group: u8) -> Byte32 {
    ckb_testtool::ckb_hash::blake2b_256([b'x', group]).pack()
}

fn alphabet(env: &Env) -> Alphabet {
    let deposit_header = gen_header(1_600, DEPOSIT_AR, 35, 1000, 1000);
    let claim_deposit_header = gen_header(1_554, GENESIS_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let value = |amount: u64| soft_capped_ickb(amount, DEPOSIT_AR);
    // The least amount whose unscaled iCKB exceeds the soft cap by 10 shannons: a 1-shannon haircut.
    let soft_cap_amount = ((SOFT_CAP + 10) * u128::from(DEPOSIT_AR)).div_ceil(u128::from(GENESIS_AR)) as u64;
    let (v_min, v_cap, v_max) = (value(MIN_DEPOSIT), value(soft_cap_amount), value(MAX_DEPOSIT));
    assert!(v_cap > SOFT_CAP && v_cap + 1 == u128::from(soft_cap_amount) * u128::from(GENESIS_AR) / u128::from(DEPOSIT_AR));

    let deposit = |lock: &Script, amount: u64| Cell {
        capacity: deposit_capacity(lock, &env.dao, 8, amount),
        lock: lock.clone(),
        type_: Some(env.dao.clone()),
        data: vec![0; 8],
    };
    let receipt = |type_: &Script, quantity: u32, amount: u64| Cell {
        capacity: occupied_capacity(&env.user, type_, 12),
        lock: env.user.clone(),
        type_: Some(type_.clone()),
        data: receipt_data(quantity, amount).to_vec(),
    };
    let udt = |xudt: &Script, amount: u128| Cell {
        capacity: occupied_capacity(&env.user, xudt, 16),
        lock: env.user.clone(),
        type_: Some(xudt.clone()),
        data: udt_data(amount).to_vec(),
    };
    let request = |amount: u64, deposit_number: u64| Cell {
        lock: env.owned_owner.clone(),
        data: deposit_number.to_le_bytes().to_vec(),
        ..deposit(&env.ickb, amount)
    };
    let owner = |type_: &Script, distance: i32| Cell {
        capacity: occupied_capacity(&env.user, type_, 4),
        lock: env.user.clone(),
        type_: Some(type_.clone()),
        data: distance.to_le_bytes().to_vec(),
    };
    let master = Cell {
        capacity: occupied_capacity(&env.user, &env.limit_order, 0),
        lock: env.user.clone(),
        type_: Some(env.limit_order.clone()),
        data: vec![],
    };
    // An order selling v_min CKB for iCKB: "fresh" holds the CKB, "full" the iCKB after a
    // complete CKB -> iCKB fill; the 1:1 reverse ratio turns one back into the other.
    let order_with = |log: u8, unused: u128, udt: u128, action: u32, index_or_distance: i32| {
        let master_hash = if action == 1 { group_hash(MINT_TX).as_slice().try_into().expect("hash") } else { [0; 32] };
        Cell {
            capacity: deposit_capacity(&env.limit_order, &env.xudt, 89, unused as u64),
            lock: env.limit_order.clone(),
            type_: Some(env.xudt.clone()),
            data: order_data_custom(udt, action, master_hash, index_or_distance.to_le_bytes(), RATIO, RATIO, log)
                .to_vec(),
        }
    };
    let order = |unused, udt, action, index_or_distance| order_with(MIN_MATCH_LOG, unused, udt, action, index_or_distance);
    // Half filled in either direction.
    let half = v_min / 2;
    let (ickb_sibling, owned_owner_sibling) = (&env.siblings[0], &env.siblings[1]);

    let pool = |name, cell, deps: &[&HeaderView]| InArch {
        name,
        cell,
        group: POOL_TX,
        block: Some(deposit_header.clone()),
        deps: deps.iter().map(|h| (*h).clone()).collect(),
        since: 0,
        claim: None,
    };
    let plain = |name, cell, group| InArch { name, cell, group, block: None, deps: vec![], since: 0, claim: None };
    let in_withdrawal_block = |name, cell| InArch { block: Some(withdraw_header.clone()), ..plain(name, cell, WITHDRAW_TX) };
    let dh = &deposit_header;
    let inputs = vec![
        pool("deposit_min", deposit(&env.ickb, MIN_DEPOSIT), &[dh]),
        pool("deposit_cap", deposit(&env.ickb, soft_cap_amount), &[dh]),
        pool("deposit_max", deposit(&env.ickb, MAX_DEPOSIT), &[dh]),
        pool("deposit_min_nohdr", deposit(&env.ickb, MIN_DEPOSIT), &[]),
        pool("receipt_1xmin", receipt(&env.ickb, 1, MIN_DEPOSIT), &[dh]),
        pool("receipt_2xmin", receipt(&env.ickb, 2, MIN_DEPOSIT), &[dh]),
        pool("receipt_1xcap", receipt(&env.ickb, 1, soft_cap_amount), &[dh]),
        pool("receipt_1xmax", receipt(&env.ickb, 1, MAX_DEPOSIT), &[dh]),
        pool("receipt_1xmin_nohdr", receipt(&env.ickb, 1, MIN_DEPOSIT), &[]),
        plain("udt_vmin", udt(&env.xudt, v_min), UDT_TX),
        plain("udt_vmin+1", udt(&env.xudt, v_min + 1), UDT_TX),
        plain("udt_half", udt(&env.xudt, half), UDT_TX),
        plain("udt_vcap", udt(&env.xudt, v_cap), UDT_TX),
        plain("udt_vmax", udt(&env.xudt, v_max), UDT_TX),
        InArch {
            deps: vec![withdraw_header.clone(), claim_deposit_header.clone()],
            since: CLAIM_SINCE,
            claim: Some(claim_deposit_header.clone()),
            ..in_withdrawal_block("owned_request", request(MIN_DEPOSIT, claim_deposit_header.number()))
        },
        in_withdrawal_block("owner_+1", owner(&env.owned_owner, 1)),
        in_withdrawal_block("owner_-1", owner(&env.owned_owner, -1)),
        plain("master", master.clone(), MINT_TX),
        plain("order_mint_fresh_+1", order(v_min, 0, 0, 1), MINT_TX),
        plain("order_mint_fresh_-1", order(v_min, 0, 0, -1), MINT_TX),
        plain("order_fresh", order(v_min, 0, 1, 0), MATCH_TX),
        plain("order_half", order(v_min - half, half, 1, 0), MATCH_TX),
        plain("order_full", order(0, v_min, 1, 0), MATCH_TX),
        InArch { group: SIBLING_TX, ..pool("sibling_deposit", deposit(ickb_sibling, MIN_DEPOSIT), &[dh]) },
    ];
    let outputs = vec![
        ("deposit_min", deposit(&env.ickb, MIN_DEPOSIT)),
        ("deposit_min-1", deposit(&env.ickb, MIN_DEPOSIT - 1)),
        ("deposit_max", deposit(&env.ickb, MAX_DEPOSIT)),
        ("deposit_max+1", deposit(&env.ickb, MAX_DEPOSIT + 1)),
        ("receipt_1xmin", receipt(&env.ickb, 1, MIN_DEPOSIT)),
        ("receipt_2xmin", receipt(&env.ickb, 2, MIN_DEPOSIT)),
        ("receipt_0xmin", receipt(&env.ickb, 0, MIN_DEPOSIT)),
        ("receipt_1xmin+1", receipt(&env.ickb, 1, MIN_DEPOSIT + 1)),
        ("receipt_1xmax", receipt(&env.ickb, 1, MAX_DEPOSIT)),
        ("udt_vmin", udt(&env.xudt, v_min)),
        ("udt_vmin+1", udt(&env.xudt, v_min + 1)),
        ("udt_vmin-1", udt(&env.xudt, v_min - 1)),
        ("udt_half", udt(&env.xudt, half)),
        ("udt_vcap", udt(&env.xudt, v_cap)),
        ("udt_vcap+1", udt(&env.xudt, v_cap + 1)),
        ("udt_vmax", udt(&env.xudt, v_max)),
        ("request_min", request(MIN_DEPOSIT, deposit_header.number())),
        ("request_cap", request(soft_cap_amount, deposit_header.number())),
        ("request_max", request(MAX_DEPOSIT, deposit_header.number())),
        ("owner_+1", owner(&env.owned_owner, 1)),
        ("owner_-1", owner(&env.owned_owner, -1)),
        ("owner_+2", owner(&env.owned_owner, 2)),
        ("master", master),
        ("order_mint_fresh_+1", order(v_min, 0, 0, 1)),
        ("order_mint_fresh_-1", order(v_min, 0, 0, -1)),
        ("order_fresh", order(v_min, 0, 1, 0)),
        ("order_full", order(0, v_min, 1, 0)),
        ("order_half", order(v_min - half, half, 1, 0)),
        ("order_full-1", order(0, v_min - 1, 1, 0)),
        ("order_full_other_info", order_with(OTHER_MIN_MATCH_LOG, 0, v_min, 1, 0)),
        ("sibling_deposit", deposit(ickb_sibling, MIN_DEPOSIT)),
        ("sibling_receipt", receipt(ickb_sibling, 1, MIN_DEPOSIT)),
        ("sibling_owner_-1", owner(owned_owner_sibling, -1)),
    ];
    // Covers three max deposits and their change.
    let funding = Cell { capacity: 4 * MAX_DEPOSIT, lock: env.funding.clone(), type_: None, data: vec![] };
    Alphabet { inputs, outputs, funding }
}

/// The number of sequences of length 0..=max over `n` letters.
fn sequences(n: usize, max: usize) -> u64 {
    (0..=max as u32).map(|k| (n as u64).pow(k)).sum()
}

/// The `index`-th sequence in length-then-lexicographic order.
fn sequence(mut index: u64, n: usize, max: usize) -> Vec<usize> {
    for len in 0..=max as u32 {
        let count = (n as u64).pow(len);
        if index < count {
            let mut s = vec![0; len as usize];
            for slot in s.iter_mut().rev() {
                *slot = (index % n as u64) as usize;
                index /= n as u64;
            }
            return s;
        }
        index -= count;
    }
    panic!("sequence index out of range");
}

fn spec(a: &Alphabet, ins: &[usize], outs: &[usize]) -> TxSpec {
    let mut header_deps: Vec<HeaderView> = Vec::new();
    let mut inputs = Vec::new();
    for (slot, &k) in ins.iter().enumerate() {
        let arch = &a.inputs[k];
        for h in &arch.deps {
            if !header_deps.iter().any(|known| known.hash() == h.hash()) {
                header_deps.push(h.clone());
            }
        }
        inputs.push(In {
            cell: arch.cell.clone(),
            out_point: OutPoint::new(group_hash(arch.group), slot as u32),
            header: arch.block.clone(),
            since: arch.since,
            witness: Bytes::new(),
        });
    }
    for (input, &k) in inputs.iter_mut().zip(ins) {
        if let Some(h) = &a.inputs[k].claim {
            let index = header_deps.iter().position(|known| known.hash() == h.hash()).expect("claim header") as u64;
            input.witness = header_dep_index_witness(index);
        }
    }
    inputs.push(In {
        cell: a.funding.clone(),
        out_point: OutPoint::new(group_hash(FUNDING_TX), 0),
        header: None,
        since: 0,
        witness: Bytes::new(),
    });
    let mut outputs: Vec<Out> = outs.iter().map(|&k| Out { cell: a.outputs[k].1.clone(), change: false }).collect();
    outputs.push(Out { cell: Cell { capacity: 0, ..a.funding.clone() }, change: true });
    TxSpec { inputs, outputs, header_deps }
}

#[derive(Default)]
struct Summary {
    enumerated: u64,
    unrealistic: u64,
    accepted: u64,
    rejected: BTreeMap<String, u64>,
    violations: Vec<String>,
    // Accepted transactions containing each archetype: one never accepted is a dead letter.
    accepted_with: BTreeMap<String, u64>,
}

struct Search {
    context: Context,
    env: Env,
    alphabet: Alphabet,
}

impl Search {
    fn new() -> Self {
        let mut context = Context::default();
        let env = Env::new(&mut context);
        let alphabet = alphabet(&env);
        Search { context, env, alphabet }
    }

    /// Every case `k` of the enumeration over these archetype subsets, input sequences outer and
    /// output sequences inner, with `k % shards == shard`.
    fn run(&mut self, ins: &[usize], outs: &[usize], (max_in, max_out, max_cells): (usize, usize, usize), (shard, shards): (u64, u64)) -> Summary {
        let list = std::env::var("ICKB_EXHAUSTIVE_LIST_ACCEPTED").is_ok();
        let mut summary = Summary::default();
        let mut k = 0u64;
        for in_index in 0..sequences(ins.len(), max_in) {
            let i: Vec<usize> = sequence(in_index, ins.len(), max_in).iter().map(|&x| ins[x]).collect();
            let out_count = sequences(outs.len(), max_out.min(max_cells - i.len()));
            // Only this shard's cases are decoded; the others just advance the counter.
            let mut out_index = (shard + shards - k % shards) % shards;
            while out_index < out_count {
                let o: Vec<usize> = sequence(out_index, outs.len(), max_out).iter().map(|&x| outs[x]).collect();
                self.case(&i, &o, list, &mut summary);
                out_index += shards;
            }
            k += out_count;
        }
        summary
    }

    fn case(&mut self, ins: &[usize], outs: &[usize], list: bool, summary: &mut Summary) {
        summary.enumerated += 1;
        let mut spec = spec(&self.alphabet, ins, outs);
        if !spec.balance() {
            summary.unrealistic += 1;
            return;
        }
        // One context serves every case: build re-creates each input at its out point.
        let tx = spec.build(&mut self.context);
        if self.context.node_rejection(&tx).is_some() {
            summary.unrealistic += 1;
            return;
        }
        match self.context.verify(&tx, FUZZ_MAX_CYCLES) {
            Err(err) => *summary.rejected.entry(error_class(&self.env, &spec, &err.to_string())).or_default() += 1,
            Ok(_) => {
                summary.accepted += 1;
                let names = self.names(ins, outs);
                if list {
                    println!("accepted: {names}");
                }
                let mut seen: Vec<String> = Vec::new();
                for name in ins.iter().map(|&k| format!("in {}", self.alphabet.inputs[k].name))
                    .chain(outs.iter().map(|&k| format!("out {}", self.alphabet.outputs[k].0)))
                {
                    if !seen.contains(&name) {
                        *summary.accepted_with.entry(name.clone()).or_default() += 1;
                        seen.push(name);
                    }
                }
                let violations = check_invariants(&self.context, &tx);
                if !violations.is_empty() {
                    summary.violations.push(format!(
                        "{names}\n  violations:\n    {}\n  transaction:\n{}",
                        violations.join("\n    "),
                        describe(&self.env, &spec)
                    ));
                }
            }
        }
    }

    fn names(&self, ins: &[usize], outs: &[usize]) -> String {
        let ins: Vec<&str> = ins.iter().map(|&k| self.alphabet.inputs[k].name).collect();
        let outs: Vec<&str> = outs.iter().map(|&k| self.alphabet.outputs[k].0).collect();
        format!("inputs {ins:?} + funding, outputs {outs:?} + change")
    }

    fn input(&self, name: &str) -> usize {
        self.alphabet.inputs.iter().position(|a| a.name == name).expect("input archetype")
    }

    fn output(&self, name: &str) -> usize {
        self.alphabet.outputs.iter().position(|a| a.0 == name).expect("output archetype")
    }
}

fn print_summary(label: &str, s: &Summary) {
    println!(
        "{label}: {} enumerated, {} unrealistic, {} rejected, {} accepted, {} violations",
        s.enumerated,
        s.unrealistic,
        s.rejected.values().sum::<u64>(),
        s.accepted,
        s.violations.len()
    );
    for (err, n) in &s.rejected {
        println!("  rejected by {err}: {n}");
    }
    for (name, n) in &s.accepted_with {
        println!("  accepted with {name}: {n}");
    }
    for v in s.violations.iter().take(20) {
        println!("VIOLATION {v}");
    }
}

/// `ICKB_EXHAUSTIVE_SHARD=i/n`, default 0/1.
fn shard() -> (u64, u64) {
    let Ok(v) = std::env::var("ICKB_EXHAUSTIVE_SHARD") else {
        return (0, 1);
    };
    let parse = |s: &str| s.trim().parse::<u64>().ok();
    match v.split_once('/').map(|(i, n)| (parse(i), parse(n))) {
        Some((Some(i), Some(n))) if i < n => (i, n),
        _ => panic!("ICKB_EXHAUSTIVE_SHARD must be i/n with i < n"),
    }
}

#[test]
#[ignore = "hours in --release; run sharded with ICKB_EXHAUSTIVE_SHARD=i/n"]
fn exhaustive_small_scope_search_keeps_protocol_invariants() {
    let mut search = Search::new();
    let ins: Vec<usize> = (0..search.alphabet.inputs.len()).collect();
    let outs: Vec<usize> = (0..search.alphabet.outputs.len()).collect();
    let (shard, shards) = shard();
    let started = std::time::Instant::now();
    let summary = search.run(&ins, &outs, (MAX_INPUTS, MAX_OUTPUTS, MAX_CELLS), (shard, shards));
    print_summary(
        &format!(
            "exhaustive shard {shard}/{shards} ({} inputs, {} outputs archetypes, {:.0?})",
            ins.len(),
            outs.len(),
            started.elapsed()
        ),
        &summary,
    );
    assert!(summary.violations.is_empty(), "{} accepted transactions break invariants", summary.violations.len());
}

// Health check for the default suite: a deposit-and-mint slice of the same enumeration must
// still produce accepted and rejected cases, so a fixture change cannot leave the search
// checking nothing.
#[test]
fn exhaustive_slice_still_accepts_and_rejects() {
    let mut search = Search::new();
    let ins = [search.input("receipt_1xmin")];
    let outs = [search.output("udt_vmin"), search.output("udt_vmin+1"), search.output("deposit_min"), search.output("receipt_1xmin")];
    let summary = search.run(&ins, &outs, (1, 2, 3), (0, 1));
    print_summary("exhaustive slice", &summary);
    assert!(summary.violations.is_empty(), "{:?}", summary.violations);
    assert!(summary.accepted > 0 && !summary.rejected.is_empty(), "slice lost its accepted or rejected cases");
}
