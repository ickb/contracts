//! Paths of the deployed binaries that no narrower test, fuzz case or chain transaction runs.
//! An instruction-level trace of the three binaries (see `../../../coverage/README.md`) listed the
//! machine code the rest of the suite never executed. These tests run every part of it that an
//! input can reach, and check the result each time:
//!
//! - Each script keeps its accounting in a B-tree map whose nodes hold 11 entries. A transaction
//!   needs 12 distinct keys to split a node and several dozen to split an inner node.
//! - A 128-bit sum only carries into its high half past 2^64.
//! - A script longer than 256 bytes, the running one or an output lock, takes a second read into heap memory.
//! - The 128-bit division of the iCKB conversion has a correction step that few operands need.
//! - The 256-bit order arithmetic only uses its upper words for amounts near their limits.
//!
//! The last section pins where each script runs out of its 516 KB heap, so the limits are known.
use super::liveness::{claim, commit, deposit, fund, input, order, point, C2U, MAX_DEPOSIT, MIN_DEPOSIT, MIN_MATCH_LOG, U2C};
use super::*;

/// Insertion orders that split B-tree nodes at their right edge, left edge and middle.
fn orders(n: u64) -> [Vec<u64>; 3] {
    let ascending: Vec<u64> = (0..n).collect();
    let descending = ascending.iter().rev().copied().collect();
    // 7 is coprime with every n used here, so this visits each index once.
    let scattered = (0..n).map(|i| i * 7 % n).collect();
    [ascending, descending, scattered]
}

/// Twelve distinct keys whose twelfth lands at `edge` of the full eleven-key node, 0 for before
/// its first key and 11 for after its last. Where it lands picks the arm of the split.
fn twelve_keys(edge: u64) -> Vec<u64> {
    (1..=11).map(|i| 2 * i).chain([2 * edge + 1]).collect()
}

/// Deposit phase 1: one pool deposit per amount, then the given receipts, funded exactly.
fn deposit_phase1(env: &Env, tag: u8, amounts: &[u64], receipts: &[(u32, u64)]) -> TxSpec {
    let mut spec = TxSpec::default();
    for &amount in amounts {
        spec.outputs.push(Out::from(deposit(env, &env.ickb, amount, vec![0; 8])));
    }
    for &(quantity, amount) in receipts {
        spec.outputs.push(Out::from(env.receipt(quantity, amount)));
    }
    let needed: u64 = spec.outputs.iter().map(|o| o.cell.capacity).sum();
    spec.inputs.push(input(env.plain(needed), point(tag, 0), None));
    spec
}

// 31 deposits of 31 different amounts with one receipt each fill 62 of the 64 outputs a
// transaction with a DAO output may have. The receipt accounting map splits its nodes.
#[test]
fn deposits_of_many_distinct_amounts_match_their_receipts() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (tag, order) in orders(31).into_iter().enumerate() {
        let amounts: Vec<u64> = order.iter().map(|i| MIN_DEPOSIT + i * CKB).collect();
        // Receipts in another order than their deposits.
        let receipts: Vec<(u32, u64)> = amounts.iter().rev().map(|&a| (1, a)).collect();
        commit(&mut context, deposit_phase1(&env, 0x20 + tag as u8, &amounts, &receipts));
    }
}

// The first split of each script's map, with the twelfth key landing at every position of the
// full node: twelve deposit amounts, twelve orders and twelve claims are accepted in each order,
// and one wrong entry among them is still refused.
#[test]
fn a_full_map_node_splits_right_wherever_the_twelfth_key_lands() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for edge in 0..=11 {
        let keys = twelve_keys(edge);

        let amounts: Vec<u64> = keys.iter().map(|k| MIN_DEPOSIT + k * CKB).collect();
        let receipts: Vec<(u32, u64)> = amounts.iter().map(|&a| (1, a)).collect();
        commit(&mut context, deposit_phase1(&env, 0x26, &amounts, &receipts));
        let mut off = receipts.clone();
        off[11].1 += 1;
        deposit_phase1(&env, 0x26, &amounts, &off).fail(&mut context, ERROR_RECEIPT_MISMATCH);

        // Orders melted with their masters, met in key order.
        let mut melt = TxSpec::default();
        for &k in &keys {
            let master = point(0x28, k as u32);
            melt.inputs.push(input(order(&env, ORDER_CKB, 0, &master), point(0x27, k as u32), None));
            melt.inputs.push(input(env.master(), master, None));
        }
        fund(&env, &mut melt);
        let mut unpaired = melt.clone();
        commit(&mut context, melt);
        // Without its master, the twelfth order has nothing to melt with.
        unpaired.inputs.remove(23);
        assert!(unpaired.balance());
        unpaired.fail(&mut context, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);

        commit(&mut context, keyed_claim(&env, &keys, None));
        // The owner of the request at key 4 points at key 6, which then has two owners.
        keyed_claim(&env, &keys, Some(4)).fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
    }
}

/// A claim of one request per key, each from the same deposit block and followed by its owner.
/// The request sits at out point index `key` and its owner 100 further; `misdirect` points the
/// owner of that key at the request two indices up.
fn keyed_claim(env: &Env, keys: &[u64], misdirect: Option<u64>) -> TxSpec {
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposit_header = gen_header(1_554, GENESIS_AR, 35, 1000, 1000);
    let mut spec = TxSpec { header_deps: vec![withdraw_header.clone(), deposit_header], ..Default::default() };
    let mut claimed = 0;
    for &key in keys {
        let request = deposit(env, &env.owned_owner, MIN_DEPOSIT, 1_554u64.to_le_bytes().to_vec());
        claimed += dao_maximum_withdraw_capacity(&request.output(), 8, GENESIS_AR, SYNTHETIC_WITHDRAW_AR);
        let owner = env.owner(if misdirect == Some(key) { -98 } else { -100 });
        claimed += owner.capacity;
        spec.inputs.push(In { since: CLAIM_SINCE, witness: header_dep_index_witness(1), ..input(request, point(0x29, key as u32), Some(&withdraw_header)) });
        spec.inputs.push(input(owner, point(0x29, 100 + key as u32), Some(&withdraw_header)));
    }
    spec.outputs.push(Out::from(env.plain(claimed)));
    spec
}

// Among 31 distinct amounts, one receipt one shannon off its deposit is still found, wherever it sits.
#[test]
fn one_receipt_off_by_a_shannon_among_many_distinct_amounts_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let amounts: Vec<u64> = (0..31).map(|i| MIN_DEPOSIT + i * CKB).collect();
    for wrong in [0, 5, 11, 12, 17, 23, 30] {
        for delta in [1u64, u64::MAX] {
            let receipts: Vec<(u32, u64)> =
                amounts.iter().enumerate().map(|(i, &a)| (1, if i == wrong { a.wrapping_add(delta) } else { a })).collect();
            deposit_phase1(&env, 0x24, &amounts, &receipts).fail(&mut context, ERROR_RECEIPT_MISMATCH);
        }
        // The same deposit counted twice by its receipt.
        let receipts: Vec<(u32, u64)> = amounts.iter().enumerate().map(|(i, &a)| (if i == wrong { 2 } else { 1 }, a)).collect();
        deposit_phase1(&env, 0x24, &amounts, &receipts).fail(&mut context, ERROR_RECEIPT_MISMATCH);
    }
}

// Receipts alone need no DAO output, so 64 outputs do not bound them: 200 receipts of distinct
// amounts split inner map nodes too, and are rejected because no deposit backs them.
#[test]
fn many_receipts_of_distinct_amounts_without_deposits_are_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for order in orders(200) {
        let receipts: Vec<(u32, u64)> = order.iter().map(|i| (1, MIN_DEPOSIT + i * CKB)).collect();
        deposit_phase1(&env, 0x25, &[], &receipts).fail(&mut context, ERROR_RECEIPT_MISMATCH);
    }
}

const WIDE_ORDERS: u32 = 150;
const ORDER_CKB: u64 = 100 * CKB;

fn mint_order(env: &Env, unused: u64, master_distance: i32) -> Cell {
    let data = order_data_custom(0, 0, [0; 32], master_distance.to_le_bytes(), C2U, U2C, MIN_MATCH_LOG).to_vec();
    Cell { capacity: deposit_capacity(&env.limit_order, &env.xudt, 89, unused), lock: env.limit_order.clone(), type_: Some(env.xudt.clone()), data }
}

/// What a taker pays in iCKB for `ckb` shannons at the order's CKB-to-iCKB ratio, rounded up.
fn c2u_price(ckb: u64) -> u128 {
    (u128::from(ckb) * u128::from(C2U.0)).div_ceil(u128::from(C2U.1))
}

/// `count` orders filled in full in one transaction; `short` pays that order one unit too little.
fn wide_match(env: &Env, tag: u8, count: u32, short: Option<u32>) -> TxSpec {
    let mut spec = TxSpec::default();
    let mut paid = 0;
    for k in 0..count {
        let master = point(tag + 1, k);
        let price = c2u_price(ORDER_CKB);
        paid += price;
        spec.inputs.push(input(order(env, ORDER_CKB, 0, &master), point(tag, k), None));
        spec.outputs.push(Out::from(order(env, 0, if short == Some(k) { price - 1 } else { price }, &master)));
    }
    spec.inputs.push(input(env.udt(paid), point(tag + 2, 0), None));
    if short.is_some() {
        spec.outputs.push(Out::from(env.udt(1)));
    }
    fund(env, &mut spec);
    spec
}

// 150 orders minted, filled and melted, 150 at a time: the order map splits leaf and inner nodes.
#[test]
fn many_orders_mint_match_and_melt_in_one_transaction() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    commit(&mut context, wide_mint(&env, WIDE_ORDERS));
    commit(&mut context, wide_match(&env, 0x30, WIDE_ORDERS, None));
    commit(&mut context, wide_melt(&env, 0x33, WIDE_ORDERS));
}

// Among 150 fills, one order paid a single unit too little is still found, wherever it sits.
#[test]
fn one_underpaid_order_among_many_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for short in [0, 11, 12, 77, WIDE_ORDERS - 1] {
        let mut spec = wide_match(&env, 0x38, WIDE_ORDERS, Some(short));
        assert!(spec.balance());
        spec.fail(&mut context, ERROR_LIMIT_ORDER_DECREASING_VALUE);
    }
}

// Among 150 minted orders, one whose master sits one cell off leaves an order and a master unpaired.
#[test]
fn one_misplaced_master_among_many_minted_orders_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let n = WIDE_ORDERS;
    for wrong in [0, 12, n - 1] {
        let mut mint = TxSpec::default();
        for k in 0..n {
            mint.outputs.push(Out::from(mint_order(&env, ORDER_CKB, if k == wrong { n as i32 + 1 } else { n as i32 })));
        }
        for _ in 0..n {
            mint.outputs.push(Out::from(env.master()));
        }
        fund(&env, &mut mint);
        assert!(mint.balance());
        mint.fail(&mut context, if wrong == n - 1 { ERROR_LIMIT_ORDER_INVALID_CONFIGURATION } else { ERROR_LIMIT_ORDER_SAME_MASTER });
    }
}

const WIDE_CLAIMS: u32 = 100;

// A claim's inputs are not bound by the 64 outputs of a withdrawal: 100 requests and their 100
// owners in one claim split leaf and inner nodes of the pairing map.
#[test]
fn many_withdrawal_requests_claim_in_one_transaction() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    commit(&mut context, claim(&env, WIDE_CLAIMS, 0));
}

// Among 100 pairs, one owner pointing at its neighbour's request leaves one request unowned and
// one owned twice.
#[test]
fn one_misdirected_owner_among_many_claims_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for wrong in [0usize, 12, 57, WIDE_CLAIMS as usize - 2] {
        let mut spec = claim(&env, WIDE_CLAIMS, 0);
        // Inputs alternate request, owner; this owner now points one request further.
        spec.inputs[2 * wrong + 1].cell.data = (-(WIDE_CLAIMS as i32) + 1).to_le_bytes().to_vec();
        spec.fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
    }
}

const TWO_63: u128 = 1 << 63;

/// Phase 2 of one receipt with extra iCKB inputs and the given iCKB outputs.
fn phase2(env: &Env, tag: u8, receipt: (u32, u64), ar: u64, udt_in: &[u128], udt_out: &[u128]) -> TxSpec {
    let header = gen_header(1_000, ar, 35, 1000, 1000);
    let mut spec = TxSpec { header_deps: vec![header.clone()], ..Default::default() };
    spec.inputs.push(input(env.receipt(receipt.0, receipt.1), point(tag, 0), Some(&header)));
    for (k, &amount) in udt_in.iter().enumerate() {
        spec.inputs.push(input(env.udt(amount), point(tag, 1 + k as u32), None));
    }
    for &amount in udt_out {
        spec.outputs.push(Out::from(env.udt(amount)));
    }
    fund(env, &mut spec);
    spec
}

/// The withdrawal of one pool deposit of `amount`, created at rate `ar`, with the given iCKB inputs and outputs.
fn withdrawal(env: &Env, tag: u8, amount: u64, ar: u64, udt_in: &[u128], udt_out: &[u128]) -> TxSpec {
    let header = gen_header(1_000, ar, 35, 1000, 1000);
    let pool_deposit = deposit(env, &env.ickb, amount, vec![0; 8]);
    let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..pool_deposit.clone() };
    let mut spec = TxSpec { header_deps: vec![header.clone()], ..Default::default() };
    spec.inputs.push(input(pool_deposit, point(tag, 0), Some(&header)));
    for (k, &udt) in udt_in.iter().enumerate() {
        spec.inputs.push(input(env.udt(udt), point(tag, 1 + k as u32), None));
    }
    spec.outputs.push(Out::from(request));
    spec.outputs.push(Out::from(env.owner(-1)));
    for &udt in udt_out {
        spec.outputs.push(Out::from(env.udt(udt)));
    }
    fund(env, &mut spec);
    spec
}

// No chain state reaches these sums: all iCKB ever issued stays below 2^64. They run the carry
// into the high half of each 128-bit sum, on both sides of the balance equation.
#[test]
fn sums_past_two_to_the_64_still_balance_exactly() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let value = soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR);
    let max = u128::from(u64::MAX);
    commit(&mut context, phase2(&env, 0x40, (1, MIN_DEPOSIT), GENESIS_AR, &[TWO_63, TWO_63], &[max, 1 + value]));
    commit(&mut context, phase2(&env, 0x41, (1, MIN_DEPOSIT), GENESIS_AR, &[max, max, max], &[max, max, max, value]));
    for (tag, outputs) in [[max, value], [max, 2 + value], [max - 1, 1 + value], [0, 1 + value]].into_iter().enumerate() {
        let mut spec = phase2(&env, 0x42 + tag as u8, (1, MIN_DEPOSIT), GENESIS_AR, &[TWO_63, TWO_63], &outputs);
        assert!(spec.balance());
        spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
    }
    // The receipt side: a quantity no transaction can back makes one receipt worth more than 2^64.
    let mut spec = phase2(&env, 0x46, (u32::MAX, MAX_DEPOSIT), GENESIS_AR, &[], &[max, max]);
    assert!(spec.balance());
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
    // The deposit side: output iCKB plus the deposit burned crosses 2^64.
    commit(&mut context, withdrawal(&env, 0x47, MIN_DEPOSIT, GENESIS_AR, &[max, value], &[max]));
    // A deposit worth more than 2^64 by itself: only a header below the genesis rate, which no
    // chain has, prices 1,000 CKB that high.
    let priced = soft_capped_ickb(MIN_DEPOSIT, 1);
    assert!(priced > max);
    commit(&mut context, withdrawal(&env, 0x4b, MIN_DEPOSIT, 1, &[priced], &[]));
    let mut spec = withdrawal(&env, 0x4c, MIN_DEPOSIT, 1, &[priced - 1], &[]);
    assert!(spec.balance());
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// A sum past 2^128 stops the script instead of wrapping around to a small number.
#[test]
fn sums_past_two_to_the_128_stop_the_script() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let value = soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR);
    // Input iCKB wraps to exactly zero, so a wrapping sum would balance against the receipt alone.
    let mut spec = phase2(&env, 0x48, (1, MIN_DEPOSIT), GENESIS_AR, &[1 << 127, 1 << 127], &[value]);
    assert!(spec.balance());
    spec.fail(&mut context, ERROR_SCRIPT_PANIC);
    // Input iCKB plus the receipt wraps to exactly the output.
    let mut spec = phase2(&env, 0x49, (1, MIN_DEPOSIT), GENESIS_AR, &[u128::MAX - value + 2], &[1]);
    assert!(spec.balance());
    spec.fail(&mut context, ERROR_SCRIPT_PANIC);
    // A receipt whose quantity times its value wraps: only a header below the genesis rate, which
    // no chain has, makes one deposit worth that much.
    let mut spec = phase2(&env, 0x4a, (u32::MAX, u64::MAX), 1, &[], &[1]);
    assert!(spec.balance());
    spec.fail(&mut context, ERROR_SCRIPT_PANIC);
}

/// The user lock with `args_len` bytes of args: its script serializes to 53 + `args_len` bytes.
fn long_lock(env: &Env, args_len: usize) -> Script {
    env.user.clone().as_builder().args(Bytes::from(vec![0x5a; args_len]).pack()).build()
}

/// Transactions that run each script with `lock` on an output: a deposit with its receipt, an
/// order mint, a withdrawal, and a claim, the one where `Owned Owner` runs without `iCKB Logic`.
fn with_output_lock(env: &Env, tag: u8, lock: &Script) -> [TxSpec; 4] {
    let mut deposit_tx = TxSpec::default();
    deposit_tx.outputs.push(Out::from(deposit(env, &env.ickb, MIN_DEPOSIT, vec![0; 8])));
    deposit_tx.outputs.push(Out::from(env.receipt(1, MIN_DEPOSIT).under(lock)));
    fund(env, &mut deposit_tx);

    let mut mint = TxSpec::default();
    mint.outputs.push(Out::from(mint_order(env, ORDER_CKB, 1)));
    mint.outputs.push(Out::from(env.master().under(lock)));
    fund(env, &mut mint);

    let header = gen_header(1_000, GENESIS_AR, 35, 1000, 1000);
    let pool_deposit = deposit(env, &env.ickb, MIN_DEPOSIT, vec![0; 8]);
    let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..pool_deposit.clone() };
    let mut withdraw = TxSpec { header_deps: vec![header.clone()], ..Default::default() };
    withdraw.inputs.push(input(pool_deposit, point(tag, 0), Some(&header)));
    withdraw.inputs.push(input(env.udt(soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR)), point(tag, 1), None));
    withdraw.outputs.push(Out::from(request));
    withdraw.outputs.push(Out::from(env.owner(-1).under(lock)));
    fund(env, &mut withdraw);

    let mut claim_tx = keyed_claim(env, &[0], None);
    claim_tx.outputs[0] = Out::from(env.plain(0).under(lock));
    fund(env, &mut claim_tx);

    [deposit_tx, mint, withdraw, claim_tx]
}

// Every script reads each output lock to check its args. A lock that serializes to at most 256
// bytes fits the first read; a longer one takes a second read into heap memory. Wallet locks are
// short, so no other test and no transaction on mainnet or testnet ran the second read.
#[test]
fn output_locks_longer_than_one_read_are_accepted() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // 203 bytes of args is the longest lock one read holds; 204 is the shortest that needs two.
    for (n, args_len) in [203, 204, 205, 300, 4_096, 100_000, 200_000].into_iter().enumerate() {
        let lock = long_lock(&env, args_len);
        for spec in with_output_lock(&env, 0x60 + n as u8, &lock) {
            commit(&mut context, spec);
        }
    }
}

// The args check still sees a script's own code under long args.
#[test]
fn own_code_under_long_args_on_an_output_lock_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (n, (script, name)) in [(&env.ickb, "ickb_logic"), (&env.limit_order, "limit_order"), (&env.owned_owner, "owned_owner")].into_iter().enumerate() {
        for args_len in [1, 203, 204, 300, 4_096] {
            let lock = script.clone().as_builder().args(Bytes::from(vec![0x5a; args_len]).pack()).build();
            let mut spec = with_output_lock(&env, 0x70, &env.user)[n].clone();
            spec.outputs.insert(spec.outputs.len() - 1, Out::from(plain(&lock, 0).sized()));
            assert!(spec.balance());
            let err = spec.verify(&mut context).expect_err(name);
            assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
        }
    }
}

// A script that runs under long args reads its own script in two reads as well, then refuses its
// args. Run as an output type and as an input lock: a cell under such a lock can never be spent,
// like any cell under these scripts with args.
#[test]
fn own_code_running_under_long_args_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for script in [&env.ickb, &env.limit_order, &env.owned_owner] {
        for args_len in [203, 204, 300, 4_096, 131_020] {
            let long = script.clone().as_builder().args(Bytes::from(vec![0x5a; args_len]).pack()).build();
            let mut as_type = TxSpec::default();
            as_type.outputs.push(Out::from(Cell { type_: Some(long.clone()), ..env.plain(0) }.sized()));
            fund(&env, &mut as_type);
            stops(&mut context, as_type, ERROR_NOT_EMPTY_ARGS);

            let mut as_lock = TxSpec::default();
            as_lock.inputs.push(In::new(plain(&long, 0).sized()));
            fund(&env, &mut as_lock);
            stops(&mut context, as_lock, ERROR_NOT_EMPTY_ARGS);
        }
    }
}

const EDGES_64: [u64; 10] = [1, 2, 3, (1 << 32) - 1, 1 << 32, (1 << 32) + 1, (1 << 63) - 1, 1 << 63, u64::MAX - 1, u64::MAX];
const EDGES_128: [u128; 8] = [0, 1, (1 << 64) - 1, 1 << 64, (1 << 64) + 1, 1 << 127, (1 << 127) + 1, u128::MAX];

/// Never zero: a ratio with a zero side is no ratio.
fn edge_64(rng: &mut Rng) -> u64 {
    if rng.chance(2, 3) { EDGES_64[rng.index(EDGES_64.len())] } else { (rng.next() >> rng.below(64)).max(1) }
}

fn edge_128(rng: &mut Rng, max: u128) -> u128 {
    let x = if rng.chance(2, 3) {
        EDGES_128[rng.index(EDGES_128.len())]
    } else {
        ((u128::from(rng.next()) << 64) | u128::from(rng.next())) >> rng.below(128)
    };
    // Past the limit, the largest value that fits, minus a little.
    if x > max { max - u128::from(rng.below(3)).min(max) } else { x }
}

fn edge_order(env: &Env, unused: u64, udt: u128, c2u: (u64, u64), u2c: (u64, u64), log: u8) -> Cell {
    let data = order_data_custom(udt, 1, [0x77; 32], 0u32.to_le_bytes(), c2u, u2c, log).to_vec();
    Cell { capacity: deposit_capacity(&env.limit_order, &env.helper, 89, unused), lock: env.limit_order.clone(), type_: Some(env.helper.clone()), data }
}

// A fill is accepted exactly when the taker pays at least the price, rounded up. The price is one
// division here; the script instead compares two 256-bit sums of products. Amounts and ratios sit
// at the edges of their 64 and 128 bits, where the sums of products fill up to 192 bits, so the two agree
// only if every carry of the deployed multiplication, addition and comparison is right. Orders of
// any token can hold such amounts.
#[test]
fn fills_at_the_limits_of_every_operand_are_priced_exactly() {
    // The most unused CKB an order holds here, above the 6.6 * 10^18 shannons issued by mainnet
    // block 20,958,587.
    const LIMIT: u64 = 1 << 63;
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let mut rng = Rng(0x5eed_0e01);
    let (mut accepted, mut rejected, mut cases) = (0, 0, 0);
    while cases < 400 {
        let (ckb_mul, udt_mul) = (edge_64(&mut rng), edge_64(&mut rng));
        let log = [0u8, 20, 33, 64][rng.index(4)];
        let min_match = 1u128 << log;
        let ckb_to_udt = rng.chance(1, 2);
        // The cells before and after the fill, the price in the paid asset, and whether the fill is too small.
        let (unused_in, udt_in, unused_at_price, udt_at_price, too_small) = if ckb_to_udt {
            // The taker takes `taken` shannons and pays iCKB-side units.
            let unused_in = edge_64(&mut rng) % LIMIT + 1;
            let taken = (edge_64(&mut rng) % unused_in).max(1);
            let price = (u128::from(taken) * u128::from(ckb_mul)).div_ceil(u128::from(udt_mul));
            let udt_in = edge_128(&mut rng, u128::MAX - price - 1);
            let partial = unused_in != taken;
            (unused_in, udt_in, unused_in - taken, udt_in + price, partial && u128::from(taken) < min_match)
        } else {
            // The taker takes `taken` units and pays shannons.
            let taken = edge_128(&mut rng, u128::MAX / u128::from(udt_mul)).max(1);
            let worth = taken * u128::from(udt_mul);
            let price = worth.div_ceil(u128::from(ckb_mul));
            if price >= u128::from(LIMIT) {
                continue;
            }
            let udt_left = edge_128(&mut rng, u128::MAX - taken);
            // The order still holds at most LIMIT once paid one unit more than the price.
            let unused_in = edge_64(&mut rng) % (LIMIT - price as u64);
            (unused_in, udt_left + taken, unused_in + price as u64, udt_left, udt_left != 0 && worth < min_match * u128::from(ckb_mul))
        };
        cases += 1;
        let (c2u, u2c) = if ckb_to_udt { ((ckb_mul, udt_mul), (0, 0)) } else { ((0, 0), (ckb_mul, udt_mul)) };
        for paid in [-1i8, 0, 1] {
            // One unit less than the price, the price, one unit more.
            let (unused_out, udt_out) = if ckb_to_udt {
                (unused_at_price, udt_at_price.wrapping_add_signed(paid.into()))
            } else {
                (unused_at_price.wrapping_add_signed(paid.into()), udt_at_price)
            };
            let mut spec = TxSpec::default();
            spec.inputs.push(In::new(edge_order(&env, unused_in, udt_in, c2u, u2c, log)));
            // Funding covers what the taker pays in CKB, so the total capacity stays within 64 bits.
            spec.inputs.push(In::new(plain(&env.funding, unused_out.saturating_sub(unused_in) + 1_000 * CKB)));
            spec.outputs.push(Out::from(edge_order(&env, unused_out, udt_out, c2u, u2c, log)));
            spec.outputs.push(Out { cell: plain(&env.funding, 0), change: true });
            assert!(spec.balance());
            let result = spec.verify(&mut context);
            let case = format!("{}: in ({unused_in}, {udt_in}) out ({unused_out}, {udt_out}) ratio ({ckb_mul}, {udt_mul}) log {log}", if ckb_to_udt { "CKB to UDT" } else { "UDT to CKB" });
            if paid < 0 {
                assert_script_error(result.expect_err(&case), ERROR_LIMIT_ORDER_DECREASING_VALUE);
                rejected += 1;
            } else if too_small {
                assert_script_error(result.expect_err(&case), ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
                rejected += 1;
            } else {
                result.unwrap_or_else(|e| panic!("{case}: {e}"));
                accepted += 1;
            }
        }
    }
    // Both verdicts must be common, or the case generator has drifted into one of them.
    assert!(accepted > 300 && rejected > 300, "accepted {accepted}, rejected {rejected}");
}

/// (amount, accumulated rate) pairs that take each branch of the 128-bit division in the iCKB
/// conversion. `../../../coverage/division_paths.py` ports the division the compiler links into
/// the scripts, searches for such pairs, and with `check` confirms each row takes the branch named.
const DIVISION_ROWS: [(&str, u64, u64); 14] = [
    // The rows a deposit within the bounds can take at a rate between genesis and double genesis.
    ("B", 55_664_639_800_631, 10_869_849_637_299_280),
    ("loop-half", 13_811_135_044_099, 18_942_608_126_162_399),
    ("loop-early0", 224_365_471_226, 19_760_549_432_632_552),
    ("loop-early1", 216_140_858_913, 19_608_266_820_388_398),
    // Below 77 CKB: only a receiptless pool deposit holds so little, see the report on those.
    ("A", 410_008_849, 11_244_928_465_014_036),
    ("A-corrected", 23_911_541, 10_000_000_000_000_011),
    ("A-corrected", 7_382_323_769, 19_373_315_692_146_907),
    ("half", 3, 10_858_395_948_649_177),
    // No chain state reaches these: more CKB than exists, or a rate no header carries.
    ("B-corrected", 5_884_432_007_570_793_220, 10_369_472_052_103_117),
    ("early1", 563, 5_471_089_347_912_617_004),
    ("loop-early0", 730_035_877_242, 1_082_152_129_030_520_160),
    ("loop-early1", 6_170_894_642, 10_000_281_385_766_517),
    ("early0", 62, 4_278_428_074_455_958_971),
    ("small-divisor", 2_000, 4_000_000_000),
];

// The quotient the script computes on RISC-V equals the one this test computes natively, and one
// unit more is refused, on every branch of the division, the two correction steps included.
#[test]
fn conversion_division_is_exact_on_every_branch() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (n, (branch, amount, ar)) in DIVISION_ROWS.into_iter().enumerate() {
        let value = soft_capped_ickb(amount, ar);
        let mut exact = phase2(&env, 0x80 + n as u8, (1, amount), ar, &[], &[value]);
        assert!(exact.balance());
        exact.verify(&mut context).unwrap_or_else(|e| panic!("{branch} ({amount}, {ar}): {e}"));
        let mut one_more = phase2(&env, 0xa0 + n as u8, (1, amount), ar, &[], &[value + 1]);
        assert!(one_more.balance());
        one_more.fail(&mut context, ERROR_AMOUNT_MISMATCH);
    }
}

/// Verifies under mainnet's block cycle limit and requires the script to stop with `err_code`.
fn stops(context: &mut Context, mut spec: TxSpec, err_code: i8) {
    assert!(spec.balance(), "the test built an unbalanceable transaction");
    let tx = spec.build(context);
    let err = context.verify(&tx, mainnet().max_block_cycles()).expect_err("the transaction should be rejected");
    assert_script_error(err, err_code);
}

fn wide_mint(env: &Env, count: u32) -> TxSpec {
    let mut mint = TxSpec::default();
    for _ in 0..count {
        mint.outputs.push(Out::from(mint_order(env, ORDER_CKB, count as i32)));
    }
    for _ in 0..count {
        mint.outputs.push(Out::from(env.master()));
    }
    fund(env, &mut mint);
    mint
}

fn wide_melt(env: &Env, tag: u8, count: u32) -> TxSpec {
    let mut melt = TxSpec::default();
    for k in 0..count {
        let master = point(tag + 1, k);
        melt.inputs.push(input(order(env, ORDER_CKB, 0, &master), point(tag, k), None));
        melt.inputs.push(input(env.master(), master, None));
    }
    fund(env, &mut melt);
    melt
}

/// The most orders one transaction can mint, fill or melt when the script meets their masters in
/// sorted order, which leaves its map nodes half empty. Scattered masters fit more.
const MAX_SORTED_ORDERS: u32 = 382;
/// The node's default limit on the cycles of a transaction it takes into its pool.
const POOL_CYCLES: u64 = 70_000_000;

// `Limit Order` keeps about 8 KB per map node and has 516 KB of heap. With 383 sorted orders the
// heap runs out and the script stops. That bounds a batch, not an order: each order still mints,
// fills and melts in a smaller transaction, and 382 stay far below the block size and the pool cycles.
#[test]
fn sorted_orders_fit_one_transaction_up_to_382() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let n = MAX_SORTED_ORDERS;
    for spec in [wide_mint(&env, n), wide_match(&env, 0xb0, n, None), wide_melt(&env, 0xb4, n)] {
        assert!(commit(&mut context, spec) < POOL_CYCLES);
    }
    for spec in [wide_mint(&env, n + 1), wide_match(&env, 0xb8, n + 1, None), wide_melt(&env, 0xbc, n + 1)] {
        stops(&mut context, spec, ERROR_SCRIPT_PANIC);
    }
    // 400 orders whose masters the script meets out of order still melt at once.
    let mut scattered = TxSpec::default();
    for k in 0..400 {
        let master = point(0xbf, k * 7 % 400);
        scattered.inputs.push(input(order(&env, ORDER_CKB, 0, &master), point(0xbe, k), None));
        scattered.inputs.push(input(env.master(), master, None));
    }
    fund(&env, &mut scattered);
    assert!(commit(&mut context, scattered) < POOL_CYCLES);
}

/// The longest args of an output lock that every script still reads: a 256 KiB script.
const MAX_LOCK_ARGS: usize = 262_091;

// A lock is read into one heap block, and the largest free block holds 256 KiB. A longer output
// lock stops whichever iCKB script runs. No wallet lock comes near, and whoever builds the
// transaction picks its output locks.
#[test]
fn output_locks_up_to_256_kib_are_read() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for spec in with_output_lock(&env, 0xc0, &long_lock(&env, MAX_LOCK_ARGS)) {
        commit(&mut context, spec);
    }
    // Each time the script that stops is the one under test: the type of the receipt, the type of
    // the master, the lock of the pool deposit, the lock of the withdrawal request.
    let stopped_at = ["Outputs[1].Type", "Outputs[1].Type", "Inputs[0].Lock", "Inputs[0].Lock"];
    for (mut spec, at) in with_output_lock(&env, 0xc4, &long_lock(&env, MAX_LOCK_ARGS + 1)).into_iter().zip(stopped_at) {
        assert!(spec.balance());
        let tx = spec.build(&mut context);
        let err = context.verify(&tx, mainnet().max_block_cycles()).expect_err("the lock is too long to read");
        assert!(err.to_string().contains(at), "{err}");
        assert_script_error(err, ERROR_SCRIPT_PANIC);
    }
}

// 500 withdrawal requests claimed at once, from one deposit block so one header dep serves them
// all, stay within what a node takes into its pool. `Owned Owner` itself has room for six times
// as many; the cycles of 1,000 claims already exceed the pool limit.
#[test]
fn five_hundred_claims_fit_the_pool_cycle_limit() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let n = 500;
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposit_header = gen_header(1_554, GENESIS_AR, 35, 1000, 1000);
    let mut spec = TxSpec { header_deps: vec![withdraw_header.clone(), deposit_header], ..Default::default() };
    let mut claimed = 0;
    for k in 0..n {
        let request = deposit(&env, &env.owned_owner, MIN_DEPOSIT, 1_554u64.to_le_bytes().to_vec());
        claimed += dao_maximum_withdraw_capacity(&request.output(), 8, GENESIS_AR, SYNTHETIC_WITHDRAW_AR);
        let owner = env.owner(-(n as i32));
        claimed += owner.capacity;
        spec.inputs.push(In { since: CLAIM_SINCE, witness: header_dep_index_witness(1), ..input(request, point(0xc8, k), Some(&withdraw_header)) });
        spec.inputs.push(input(owner, point(0xc8, n + k), Some(&withdraw_header)));
    }
    spec.outputs.push(Out::from(env.plain(claimed)));
    assert!(commit(&mut context, spec) < POOL_CYCLES);
}

/// The most distinct pairing keys `Owned Owner` holds for one side of a transaction.
const MAX_PAIRING_KEYS: i32 = 3_063;

// `Owned Owner` runs out of heap too, far past any claim a node relays: owner cells pointing at
// 3,064 different places stop the script, where 3,063 get the pairing error they deserve. Both
// refuse the transaction.
#[test]
fn owned_owner_holds_3063_pairing_keys() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owners = |count: i32| {
        let mut spec = TxSpec::default();
        for k in 0..count {
            spec.outputs.push(Out::from(env.owner(10_000 + k)));
        }
        fund(&env, &mut spec);
        spec
    };
    stops(&mut context, owners(MAX_PAIRING_KEYS), ERROR_OWNED_OWNER_MISMATCH);
    stops(&mut context, owners(MAX_PAIRING_KEYS + 1), ERROR_SCRIPT_PANIC);
}

// `iCKB Logic` keeps one small map entry per distinct amount, so its heap outlasts the block
// size: 3,870 receipts of distinct amounts take about a whole block, and the receipt check still answers.
#[test]
fn ickb_logic_holds_a_block_of_distinct_receipt_amounts() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let bare = env.user.clone().as_builder().args(Bytes::new().pack()).build();
    let mut spec = TxSpec::default();
    for i in 0..3_870 {
        spec.outputs.push(Out::from(env.receipt(1, MIN_DEPOSIT + i).under(&bare)));
    }
    fund(&env, &mut spec);
    stops(&mut context, spec, ERROR_RECEIPT_MISMATCH);
}
