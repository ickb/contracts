//! Liveness: every state a user can reach stays spendable by that user's normal next transaction,
//! inside one mainnet block. Each test builds the largest or most extreme state the scripts and the
//! node allow, then verifies the next step, which also runs the node's verifiers and the
//! invariant oracle.
use super::*;

/// The most pairs one withdrawal can create: each request needs its owner, and a transaction with
/// a DAO output may have at most 64 outputs.
const MAX_WITHDRAWAL_PAIRS: u32 = 32;
/// The largest receipt quantity one deposit transaction can create: its deposits and the receipt
/// share the 64 outputs.
const MAX_RECEIPT_QUANTITY: u32 = 63;
const MAX_DEPOSIT: u64 = 1_000_000 * CKB;
const MIN_DEPOSIT: u64 = 1_000 * CKB;

/// Verifies like the node, whose checks include the mainnet block size limit, within mainnet's
/// block cycle limit, then requires the invariants to hold.
fn commit(context: &mut Context, mut spec: TxSpec) -> u64 {
    assert!(spec.balance(), "the test built an unbalanceable transaction");
    let tx = spec.build(context);
    let cycles = context.verify(&tx, mainnet().max_block_cycles()).unwrap_or_else(|e| panic!("rejected: {e}"));
    let violations = check_invariants(context, &tx);
    assert!(violations.is_empty(), "{violations:?}");
    cycles
}

fn point(tag: u8, index: u32) -> OutPoint {
    OutPoint::new([tag; 32].pack(), index)
}

fn deposit(env: &Env, lock: &Script, amount: u64, data: Vec<u8>) -> Cell {
    Cell { capacity: deposit_capacity(lock, &env.dao, 8, amount), lock: lock.clone(), type_: Some(env.dao.clone()), data }
}

fn input(cell: Cell, out_point: OutPoint, header: Option<&HeaderView>) -> In {
    In { cell, out_point, header: header.cloned(), since: 0, witness: Bytes::new() }
}

/// A funding input and its change output, so `balance` settles capacity as a wallet would.
fn fund(env: &Env, spec: &mut TxSpec) {
    spec.inputs.push(input(Cell { capacity: 2_000_000 * CKB, lock: env.funding.clone(), type_: None, data: vec![] }, point(0xfe, 0), None));
    spec.outputs.push(Out { cell: Cell { capacity: 0, lock: env.funding.clone(), type_: None, data: vec![] }, change: true });
}

/// Unused capacity whose iCKB value sits exactly at the 100,000 iCKB soft cap at this rate.
fn soft_cap(ar: u64) -> u64 {
    (u128::from(100_000 * CKB) * u128::from(ar) / u128::from(GENESIS_AR)) as u64
}

/// Headers no input needs, to push the needed ones past header dep index 255.
fn filler_headers(count: u64) -> Vec<HeaderView> {
    (0..count).map(|i| gen_header(1_600 + i, GENESIS_AR, 35, 1000, 1000)).collect()
}

/// One withdrawal of `pairs` pool deposits of the largest amount, each from its own block, with
/// `fillers` header deps placed before theirs. All iCKB comes from one xUDT input and no change
/// output is needed, so 32 pairs fill exactly the 64 outputs.
fn withdrawal(env: &Env, pairs: u32, fillers: u64) -> TxSpec {
    let mut spec = TxSpec { header_deps: filler_headers(fillers), ..Default::default() };
    let mut value = 0;
    for k in 0..pairs {
        let ar = GENESIS_AR + u64::from(k);
        let header = gen_header(1_000 + u64::from(k), ar, 35, 1000, 1000);
        let cell = deposit(env, &env.ickb, MAX_DEPOSIT, vec![0; 8]);
        value += soft_capped_ickb(MAX_DEPOSIT, ar);
        let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..cell.clone() };
        spec.inputs.push(input(cell, point(1, k), Some(&header)));
        spec.outputs.insert(k as usize, Out::from(request));
        spec.outputs.push(Out::from(env.owner(-(pairs as i32))));
        spec.header_deps.push(header);
    }
    let owners: u64 = spec.outputs[pairs as usize..].iter().map(|o| o.cell.capacity).sum();
    spec.inputs.push(input(Cell { capacity: owners, ..env.udt(value) }, point(2, 0), None));
    spec
}

/// The claim of `pairs` withdrawal requests created by one withdrawal (requests at 0.., owners
/// after them), with deposit headers first and `fillers` header deps after.
fn claim(env: &Env, pairs: u32, fillers: u64) -> TxSpec {
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let mut spec = TxSpec { header_deps: vec![withdraw_header.clone()], ..Default::default() };
    let mut claimed = 0;
    for k in 0..pairs {
        // The deposit blocks the fuzzer's claim uses, which CLAIM_SINCE satisfies.
        let number = 1_554 + u64::from(k);
        let request = deposit(env, &env.owned_owner, MAX_DEPOSIT, number.to_le_bytes().to_vec());
        claimed += dao_maximum_withdraw_capacity(&request.output(), 8, GENESIS_AR, SYNTHETIC_WITHDRAW_AR);
        let owner = env.owner(-(pairs as i32));
        claimed += owner.capacity;
        spec.header_deps.push(gen_header(number, GENESIS_AR, 35, 1000, 1000));
        let witness = header_dep_index_witness(spec.header_deps.len() as u64 - 1);
        spec.inputs.push(In { since: CLAIM_SINCE, witness, ..input(request, point(3, k), Some(&withdraw_header)) });
        spec.inputs.push(input(owner, point(3, pairs + k), Some(&withdraw_header)));
    }
    spec.header_deps.extend(filler_headers(fillers));
    spec.outputs.push(Out::from(env.plain(claimed)));
    spec
}

#[test]
fn largest_withdrawal_and_its_claim_each_fit_one_block() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let withdraw = commit(&mut context, withdrawal(&env, MAX_WITHDRAWAL_PAIRS, 0));
    let claim = commit(&mut context, claim(&env, MAX_WITHDRAWAL_PAIRS, 0));
    eprintln!("{MAX_WITHDRAWAL_PAIRS} pairs: withdrawal {withdraw} cycles, claim {claim} cycles");
}

// The iCKB scripts find each input's header through the input itself, never through a header dep
// index, so a long header dep list neither blocks a withdrawal nor a claim whose deposit headers
// come first.
#[test]
fn header_deps_past_index_255_block_neither_withdrawal_nor_claim() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let spec = withdrawal(&env, 4, 300);
    assert!(spec.header_deps.len() > 300);
    commit(&mut context, spec);
    commit(&mut context, claim(&env, 4, 300));
}

// The largest receipt one deposit transaction can create, then that receipt and the soft-cap edges
// redeemed at the lowest rate, one above, double, and the largest a header can carry.
#[test]
fn receipts_redeem_at_soft_cap_and_rate_edges() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let mut create = TxSpec::default();
    for _ in 0..MAX_RECEIPT_QUANTITY {
        create.outputs.push(Out::from(deposit(&env, &env.ickb, MAX_DEPOSIT, vec![0; 8])));
    }
    create.outputs.push(Out::from(env.receipt(MAX_RECEIPT_QUANTITY, MAX_DEPOSIT)));
    let needed: u64 = create.outputs.iter().map(|o| o.cell.capacity).sum();
    create.inputs.push(input(env.plain(needed), point(4, 0), None));
    commit(&mut context, create);

    for (n, ar) in [GENESIS_AR, GENESIS_AR + 1, 2 * GENESIS_AR, u64::MAX].into_iter().enumerate() {
        let edges = [MIN_DEPOSIT, soft_cap(ar) - 1, soft_cap(ar), soft_cap(ar) + 1, MAX_DEPOSIT];
        for (m, amount) in edges.into_iter().filter(|a| (MIN_DEPOSIT..=MAX_DEPOSIT).contains(a)).enumerate() {
            let header = gen_header(1_000 + n as u64, ar, 35, 1000, 1000);
            let mut spec = TxSpec { header_deps: vec![header.clone()], ..Default::default() };
            spec.inputs.push(input(env.receipt(MAX_RECEIPT_QUANTITY, amount), point(5, (n * 8 + m) as u32), Some(&header)));
            spec.outputs.push(Out::from(env.udt(u128::from(MAX_RECEIPT_QUANTITY) * soft_capped_ickb(amount, ar))));
            fund(&env, &mut spec);
            commit(&mut context, spec);
        }
    }
}

// The same edges for a pool deposit's withdrawal, which burns its iCKB value.
#[test]
fn deposits_withdraw_at_soft_cap_and_rate_edges() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (n, ar) in [GENESIS_AR, GENESIS_AR + 1, 2 * GENESIS_AR, u64::MAX].into_iter().enumerate() {
        let edges = [MIN_DEPOSIT, soft_cap(ar) - 1, soft_cap(ar), soft_cap(ar) + 1, MAX_DEPOSIT];
        for (m, amount) in edges.into_iter().filter(|a| (MIN_DEPOSIT..=MAX_DEPOSIT).contains(a)).enumerate() {
            let header = gen_header(1_000 + n as u64, ar, 35, 1000, 1000);
            let cell = deposit(&env, &env.ickb, amount, vec![0; 8]);
            let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..cell.clone() };
            let tag = (n * 8 + m) as u32;
            let mut spec = TxSpec { header_deps: vec![header.clone()], ..Default::default() };
            spec.inputs.push(input(cell, point(6, tag), Some(&header)));
            spec.inputs.push(input(env.udt(soft_capped_ickb(amount, ar)), point(7, tag), None));
            spec.outputs.push(Out::from(request));
            spec.outputs.push(Out::from(env.owner(-1)));
            fund(&env, &mut spec);
            commit(&mut context, spec);
        }
    }
}

const C2U: (u64, u64) = (3, 5);
// 3 * 9 >= 5 * 5: a round trip never loses value, which the script requires of every two-way order.
const U2C: (u64, u64) = (5, 9);
// 2^33 shannons, about 86 CKB.
const MIN_MATCH_LOG: u8 = 33;

fn order(env: &Env, unused: u64, amount: u128, master: &OutPoint) -> Cell {
    let index: u32 = master.index().unpack();
    let data = order_data_custom(amount, 1, master.tx_hash().as_slice().try_into().expect("tx hash"), index.to_le_bytes(), C2U, U2C, MIN_MATCH_LOG).to_vec();
    Cell { capacity: deposit_capacity(&env.limit_order, &env.xudt, 89, unused), lock: env.limit_order.clone(), type_: Some(env.xudt.clone()), data }
}

// Fulfilled and dust orders: no CKB left to sell, no iCKB left to sell, or neither. The owner melts
// each with its master and takes back what is left.
#[test]
fn fulfilled_and_dust_orders_melt_with_their_master() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (tag, (unused, amount)) in [(0, 5 * CKB as u128), (5 * CKB, 0), (0, 0)].into_iter().enumerate() {
        let tag = tag as u8 + 8;
        let master_point = point(tag, 1);
        let mut spec = TxSpec::default();
        spec.inputs.push(input(order(&env, unused, amount, &master_point), point(tag, 0), None));
        spec.inputs.push(input(env.master(), master_point, None));
        if amount > 0 {
            spec.outputs.push(Out::from(env.udt(amount)));
        }
        fund(&env, &mut spec);
        commit(&mut context, spec);
    }
}

// What is left below the minimum match can still be taken in full, in both directions; the
// minimum binds only partial matches.
#[test]
fn order_remainder_below_the_minimum_match_can_be_filled_in_full() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let min_match = 1u64 << MIN_MATCH_LOG;

    // CKB to iCKB: min_match - 1 shannons of CKB left, all taken, paid at the order's ratio rounded up.
    let master_point = point(12, 1);
    let left = min_match - 1;
    let paid = (u128::from(left) * u128::from(C2U.0)).div_ceil(u128::from(C2U.1));
    let mut spec = TxSpec::default();
    spec.inputs.push(input(order(&env, left, 0, &master_point), point(12, 0), None));
    spec.inputs.push(input(env.udt(paid), point(13, 0), None));
    spec.outputs.push(Out::from(order(&env, 0, paid, &master_point)));
    fund(&env, &mut spec);
    commit(&mut context, spec);

    // iCKB to CKB: iCKB worth less than the minimum match left, all taken, paid in CKB rounded up.
    let master_point = point(14, 1);
    let left = u128::from(min_match) * u128::from(U2C.0) / u128::from(U2C.1) / 2;
    let paid = (left * u128::from(U2C.1)).div_ceil(u128::from(U2C.0)) as u64;
    let mut spec = TxSpec::default();
    spec.inputs.push(input(order(&env, 0, left, &master_point), point(14, 0), None));
    spec.outputs.push(Out::from(order(&env, paid, 0, &master_point)));
    spec.outputs.push(Out::from(env.udt(left)));
    fund(&env, &mut spec);
    commit(&mut context, spec);
}
