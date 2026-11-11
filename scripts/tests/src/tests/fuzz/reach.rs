//! Paths of the deployed binaries that no narrower test, fuzz case or chain transaction runs.
//! An instruction-level trace of the three binaries (see `../../../coverage/README.md`) listed the
//! machine code the rest of the suite never executed. These tests run every part of it that an
//! input can reach and check the result each time:
//!
//! - Each script keeps its accounting in a B-tree map whose nodes hold 11 entries. A transaction
//!   needs 12 distinct keys to split a node and several dozen to split an inner node.
//! - A 128-bit sum only carries into its high half past 2^64.
//! - A script longer than 256 bytes, the running one or an output lock, takes a second read into heap memory.
//! - The 256-bit order arithmetic only uses its upper words for amounts near their limits.
//! - The 128-bit division of the iCKB conversion has a correction step that few operands need.
//!
//! The last tests pin where each script runs out of its 516 KB heap, so the limits are known.
use super::liveness::{commit, fund, input, order, point, C2U, MAX_DEPOSIT, MIN_DEPOSIT, MIN_MATCH_LOG, U2C};
use super::*;
use std::iter::once;

/// Verifies under the mainnet block cycle limit, requires the scripts to refuse with `err_code`
/// and returns the error.
fn refuse(context: &mut Context, mut spec: TxSpec, err_code: i8) -> String {
    assert!(spec.balance(), "the test built an unbalanceable transaction");
    let tx = spec.build(context);
    let err = context.verify(&tx, mainnet().max_block_cycles()).expect_err("the transaction should be refused");
    let text = err.to_string();
    assert_script_error(err, err_code);
    text
}

/// These inputs and outputs, plus the input and change output a wallet adds to fund them.
fn funded(env: &Env, inputs: impl IntoIterator<Item = In>, outputs: impl IntoIterator<Item = Cell>) -> TxSpec {
    let mut spec = TxSpec { inputs: inputs.into_iter().collect(), outputs: outputs.into_iter().map(Out::from).collect(), header_deps: vec![] };
    fund(env, &mut spec);
    spec
}

/// Insertion orders that split B-tree nodes at their right edge, left edge and middle.
fn orders(n: u32) -> [Vec<u32>; 3] {
    let ascending: Vec<u32> = (0..n).collect();
    let descending = ascending.iter().rev().copied().collect();
    // 7 is coprime with every n used here, so this visits each index once.
    let scattered = (0..n).map(|i| i * 7 % n).collect();
    [ascending, descending, scattered]
}

/// Twelve distinct keys whose twelfth lands at `edge` of the full eleven-key node, 0 for before
/// its first key and 11 for after its last. Where it lands picks the arm of the split.
fn twelve_keys(edge: u32) -> Vec<u32> {
    (1..=11).map(|i| 2 * i).chain([2 * edge + 1]).collect()
}

/// A deposit amount per key, one CKB apart.
fn amount(key: u32) -> u64 {
    MIN_DEPOSIT + u64::from(key) * CKB
}

/// Deposit phase 1: one pool deposit per amount, then the given receipts.
fn deposits(env: &Env, amounts: &[u64], receipts: &[(u32, u64)]) -> TxSpec {
    let pool_deposits = amounts.iter().map(|&a| env.deposit(&env.ickb, a));
    funded(env, [], pool_deposits.chain(receipts.iter().map(|&(quantity, a)| env.receipt(quantity, a))))
}

const ORDER_CKB: u64 = 100 * CKB;
/// What a taker pays in iCKB for the `ORDER_CKB` of one order, at its ratio, rounded up.
const PRICE: u128 = (ORDER_CKB as u128 * C2U.0 as u128).div_ceil(C2U.1 as u128);

/// The out point of the master of order `key`. All masters share one transaction hash, so their
/// map keys sort by `key`.
fn master_at(key: u32) -> OutPoint {
    point(0, key)
}

fn mint_order(env: &Env, master_distance: i32) -> Cell {
    let data = order_data_custom(0, 0, [0; 32], master_distance.to_le_bytes(), C2U, U2C, MIN_MATCH_LOG).to_vec();
    Cell { capacity: deposit_capacity(&env.limit_order, &env.xudt, 89, ORDER_CKB), lock: env.limit_order.clone(), type_: Some(env.xudt.clone()), data }
}

/// `n` orders minted at once, each `n` outputs before its master.
fn mint(env: &Env, n: u32) -> TxSpec {
    let orders = (0..n).map(|_| mint_order(env, n as i32));
    funded(env, [], orders.chain((0..n).map(|_| env.master())))
}

/// `n` orders filled in full, each paid `PRICE`.
fn fill(env: &Env, n: u32) -> TxSpec {
    let orders = (0..n).map(|k| In::new(order(env, ORDER_CKB, 0, &master_at(k))));
    let payment = In::new(env.udt(PRICE * u128::from(n)));
    funded(env, orders.chain([payment]), (0..n).map(|k| order(env, 0, PRICE, &master_at(k))))
}

/// The orders of these keys, each melted with its master.
fn melt(env: &Env, keys: impl IntoIterator<Item = u32>) -> TxSpec {
    let pairs = keys.into_iter().flat_map(|k| [In::new(order(env, ORDER_CKB, 0, &master_at(k))), input(env.master(), master_at(k), None)]);
    funded(env, pairs, [])
}

/// How far past its request an owner sits in `claim`, beyond any key used here.
const OWNER_GAP: u32 = 1_000;

/// A claim of one withdrawal request per key, all deposited in one block, so one header dep
/// serves them. Inputs alternate request and owner: the request sits at out point index `key`
/// and its owner `OWNER_GAP` further, both under one transaction hash.
fn claim(env: &Env, keys: &[u32]) -> TxSpec {
    let withdrawn = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposited = gen_header(1_554, GENESIS_AR, 35, 1000, 1000);
    let pairs = keys.iter().flat_map(|&key| {
        let request = Cell { data: deposited.number().to_le_bytes().to_vec(), ..env.deposit(&env.owned_owner, MIN_DEPOSIT) };
        let owner = env.owner(-(OWNER_GAP as i32));
        [
            In { since: CLAIM_SINCE, witness: header_dep_index_witness(1), ..input(request, point(0, key), Some(&withdrawn)) },
            input(owner, point(0, OWNER_GAP + key), Some(&withdrawn)),
        ]
    });
    let spec = funded(env, pairs, []);
    TxSpec { header_deps: vec![withdrawn, deposited], ..spec }
}

/// `claim` with the owner of `keys[i]` pointing at the request of `keys[i + 1]`, which then has
/// two owners while the request of `keys[i]` has none.
fn misdirected_claim(env: &Env, keys: &[u32], i: usize) -> TxSpec {
    let mut spec = claim(env, keys);
    spec.inputs[2 * i + 1].cell = env.owner(keys[i + 1] as i32 - (OWNER_GAP + keys[i]) as i32);
    spec
}

// The first split of the map of each script, with the twelfth key landing at every position of the
// full node: twelve deposit amounts, twelve orders and twelve claims are accepted in each order,
// and one wrong entry among them is still refused.
#[test]
fn a_full_map_node_splits_right_wherever_the_twelfth_key_lands() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for edge in 0..=11 {
        let keys = twelve_keys(edge);

        let amounts: Vec<u64> = keys.iter().map(|&k| amount(k)).collect();
        let mut receipts: Vec<(u32, u64)> = amounts.iter().map(|&a| (1, a)).collect();
        commit(&mut context, deposits(&env, &amounts, &receipts));
        receipts[11].1 += 1;
        refuse(&mut context, deposits(&env, &amounts, &receipts), ERROR_RECEIPT_MISMATCH);

        commit(&mut context, melt(&env, keys.iter().copied()));
        // Without its master, the twelfth order has nothing to melt with.
        let mut unpaired = melt(&env, keys.iter().copied());
        unpaired.inputs.remove(23);
        refuse(&mut context, unpaired, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);

        commit(&mut context, claim(&env, &keys));
        // The owner of the request at key 4 points at key 6.
        refuse(&mut context, misdirected_claim(&env, &keys, 1), ERROR_OWNED_OWNER_MISMATCH);
    }
}

// 31 deposits of 31 different amounts with one receipt each and the change fit the 64 outputs a
// transaction with a DAO output may have. They split nodes of the receipt accounting map. One
// receipt off among them is still found, wherever it sits. Receipts alone need no DAO output, so
// 200 of distinct amounts split inner nodes too. No deposit backs them, so they are refused.
#[test]
fn receipts_of_many_distinct_amounts_match_their_deposits_exactly() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for keys in orders(31) {
        let amounts: Vec<u64> = keys.iter().map(|&k| amount(k)).collect();
        // Receipts in another order than their deposits.
        let receipts: Vec<(u32, u64)> = amounts.iter().rev().map(|&a| (1, a)).collect();
        commit(&mut context, deposits(&env, &amounts, &receipts));
    }

    let amounts: Vec<u64> = (0..31).map(amount).collect();
    let receipts: Vec<(u32, u64)> = amounts.iter().map(|&a| (1, a)).collect();
    for wrong in [0, 5, 11, 12, 17, 23, 30] {
        let a = amounts[wrong];
        // One shannon more, one less and the same deposit counted twice.
        for receipt in [(1, a + 1), (1, a - 1), (2, a)] {
            let mut off = receipts.clone();
            off[wrong] = receipt;
            refuse(&mut context, deposits(&env, &amounts, &off), ERROR_RECEIPT_MISMATCH);
        }
    }

    for keys in orders(200) {
        let receipts: Vec<(u32, u64)> = keys.iter().map(|&k| (1, amount(k))).collect();
        refuse(&mut context, deposits(&env, &[], &receipts), ERROR_RECEIPT_MISMATCH);
    }
}

// 150 orders minted, filled and melted, 150 at a time: the order map splits leaf and inner nodes.
// One wrong order among them is still found, wherever it sits.
#[test]
fn many_orders_mint_fill_and_melt_in_one_transaction() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let n = 150;
    commit(&mut context, mint(&env, n));
    commit(&mut context, fill(&env, n));
    commit(&mut context, melt(&env, 0..n));

    for k in [0, 11, 12, 77, n - 1] {
        let mut underpaid = fill(&env, n);
        // The taker keeps one unit of what this order is due.
        underpaid.outputs[k as usize] = Out::from(order(&env, 0, PRICE - 1, &master_at(k)));
        underpaid.outputs.push(Out::from(env.udt(1)));
        refuse(&mut context, underpaid, ERROR_LIMIT_ORDER_DECREASING_VALUE);
    }

    for k in [0, 12, n - 1] {
        let mut misplaced = mint(&env, n);
        // This order points one cell past its master: at the master of the next order, which then
        // has two, or past the last master at a cell that is none.
        misplaced.outputs[k as usize] = Out::from(mint_order(&env, n as i32 + 1));
        let err_code = if k == n - 1 { ERROR_LIMIT_ORDER_INVALID_CONFIGURATION } else { ERROR_LIMIT_ORDER_SAME_MASTER };
        refuse(&mut context, misplaced, err_code);
    }
}

// The inputs of a claim are not bound by the 64 outputs of a withdrawal: 100 requests and their
// 100 owners in one claim split leaf and inner nodes of the pairing map. One owner pointing at the
// request of its neighbour is still found, wherever it sits.
#[test]
fn many_withdrawal_requests_claim_in_one_transaction() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let keys: Vec<u32> = (0..100).collect();
    commit(&mut context, claim(&env, &keys));
    for i in [0, 12, 57, 98] {
        refuse(&mut context, misdirected_claim(&env, &keys, i), ERROR_OWNED_OWNER_MISMATCH);
    }
}

/// The receipt of `quantity` deposits of `amount` at rate `ar` redeemed, with these iCKB cells
/// in and out.
fn redeem(env: &Env, (quantity, amount): (u32, u64), ar: u64, udt_in: &[u128], udt_out: &[u128]) -> TxSpec {
    let header = gen_header(1_000, ar, 35, 1000, 1000);
    let receipt = In::new(env.receipt(quantity, amount)).in_block(&header);
    let spec = funded(env, once(receipt).chain(udt_in.iter().map(|&a| In::new(env.udt(a)))), udt_out.iter().map(|&a| env.udt(a)));
    TxSpec { header_deps: vec![header], ..spec }
}

/// One pool deposit of `amount` at rate `ar` withdrawn, with these iCKB cells in and out.
fn withdrawal(env: &Env, amount: u64, ar: u64, udt_in: &[u128], udt_out: &[u128]) -> TxSpec {
    let header = gen_header(1_000, ar, 35, 1000, 1000);
    let pool_deposit = env.deposit(&env.ickb, amount);
    let request = Cell { lock: env.owned_owner.clone(), data: header.number().to_le_bytes().to_vec(), ..pool_deposit.clone() };
    let inputs = once(In::new(pool_deposit).in_block(&header)).chain(udt_in.iter().map(|&a| In::new(env.udt(a))));
    let spec = funded(env, inputs, [request, env.owner(-1)].into_iter().chain(udt_out.iter().map(|&a| env.udt(a))));
    TxSpec { header_deps: vec![header], ..spec }
}

// No chain state reaches these sums: all iCKB ever issued stays below 2^64. They run the carry
// into the high half of each 128-bit sum, on both sides of the balance equation.
#[test]
fn sums_past_two_to_the_64_still_balance_exactly() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let one = (1, MIN_DEPOSIT);
    let value = soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR);
    let (max, two_63) = (u128::from(u64::MAX), 1 << 63);
    commit(&mut context, redeem(&env, one, GENESIS_AR, &[two_63, two_63], &[max, 1 + value]));
    commit(&mut context, redeem(&env, one, GENESIS_AR, &[max, max, max], &[max, max, max, value]));
    for outputs in [[max, value], [max, 2 + value], [max - 1, 1 + value], [0, 1 + value]] {
        refuse(&mut context, redeem(&env, one, GENESIS_AR, &[two_63, two_63], &outputs), ERROR_AMOUNT_MISMATCH);
    }
    // The receipt side: a quantity no transaction can back makes one receipt worth more than 2^64.
    refuse(&mut context, redeem(&env, (u32::MAX, MAX_DEPOSIT), GENESIS_AR, &[], &[max, max]), ERROR_AMOUNT_MISMATCH);
    // The deposit side: output iCKB plus the deposit burned crosses 2^64.
    commit(&mut context, withdrawal(&env, MIN_DEPOSIT, GENESIS_AR, &[max, value], &[max]));
    // A deposit worth more than 2^64 by itself: only a header below the genesis rate, which no
    // chain has, prices 1,000 CKB that high.
    let priced = soft_capped_ickb(MIN_DEPOSIT, 1);
    assert!(priced > max);
    commit(&mut context, withdrawal(&env, MIN_DEPOSIT, 1, &[priced], &[]));
    refuse(&mut context, withdrawal(&env, MIN_DEPOSIT, 1, &[priced - 1], &[]), ERROR_AMOUNT_MISMATCH);
}

// A sum past 2^128 stops the script instead of wrapping around to a small number.
#[test]
fn sums_past_two_to_the_128_stop_the_script() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let one = (1, MIN_DEPOSIT);
    let value = soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR);
    // Input iCKB wraps to exactly zero, so a wrapping sum would balance against the receipt alone.
    refuse(&mut context, redeem(&env, one, GENESIS_AR, &[1 << 127, 1 << 127], &[value]), ERROR_SCRIPT_PANIC);
    // Input iCKB plus the receipt wraps to exactly the output.
    refuse(&mut context, redeem(&env, one, GENESIS_AR, &[u128::MAX - value + 2], &[1]), ERROR_SCRIPT_PANIC);
    // A receipt whose quantity times its value wraps: only a header below the genesis rate, which
    // no chain has, makes one deposit worth that much.
    refuse(&mut context, redeem(&env, (u32::MAX, u64::MAX), 1, &[], &[1]), ERROR_SCRIPT_PANIC);
}

/// `script` with `args_len` bytes of args: it serializes to 53 + `args_len` bytes.
fn with_args(script: &Script, args_len: usize) -> Script {
    script.clone().as_builder().args(Bytes::from(vec![0x5a; args_len]).pack()).build()
}

/// One transaction for each way the iCKB scripts run. A deposit with its receipt runs `iCKB Logic`,
/// an order mint `Limit Order`, a withdrawal `iCKB Logic` and `Owned Owner`. A claim runs `Owned
/// Owner` without `iCKB Logic`.
fn each_script_runs(env: &Env) -> [TxSpec; 4] {
    [
        deposits(env, &[MIN_DEPOSIT], &[(1, MIN_DEPOSIT)]),
        mint(env, 1),
        withdrawal(env, MIN_DEPOSIT, GENESIS_AR, &[soft_capped_ickb(MIN_DEPOSIT, GENESIS_AR)], &[]),
        claim(env, &[0]),
    ]
}

/// The same transaction with one more output, under `lock`.
fn with_output_lock(mut spec: TxSpec, lock: &Script) -> TxSpec {
    spec.outputs.push(Out::from(plain(lock, 0).sized()));
    spec
}

// Every script reads each output lock to check its args. A lock that serializes to at most 256
// bytes fits the first read; a longer one takes a second read into heap memory, whose largest free
// block holds 256 KiB. Wallet locks are short, so no other test and no transaction on mainnet or
// testnet ran the second read. A longer lock stops whichever iCKB script runs; whoever builds the
// transaction picks its output locks.
#[test]
fn output_locks_up_to_256_kib_are_read() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // 203 bytes of args is the longest lock one read holds and 204 the shortest that needs two.
    // 262,091 make a 256 KiB script.
    for args_len in [203, 204, 205, 300, 4_096, 100_000, 200_000, 262_091] {
        for spec in each_script_runs(&env) {
            commit(&mut context, with_output_lock(spec, &with_args(&env.user, args_len)));
        }
    }
    // Each time the script that stops is the one under test: the type of the receipt, the type of
    // the master, the lock of the pool deposit, the lock of the withdrawal request.
    let stopped_at = ["Outputs[1].Type", "Outputs[1].Type", "Inputs[0].Lock", "Inputs[0].Lock"];
    for (spec, at) in each_script_runs(&env).into_iter().zip(stopped_at) {
        let err = refuse(&mut context, with_output_lock(spec, &with_args(&env.user, 262_092)), ERROR_SCRIPT_PANIC);
        assert!(err.contains(at), "{err}");
    }
}

// A script refuses its own code with args, read in one read or two: on an output lock of a
// transaction it runs in, as an output type and as an input lock. A cell under such a lock can
// never be spent, like any cell under these scripts with args.
#[test]
fn own_code_with_args_is_refused_however_long_the_args() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // The first three of `each_script_runs` run these scripts in turn.
    for (script, spec) in [&env.ickb, &env.limit_order, &env.owned_owner].into_iter().zip(each_script_runs(&env)) {
        for args_len in [1, 203, 204, 300, 4_096, 131_020] {
            let own = with_args(script, args_len);
            refuse(&mut context, with_output_lock(spec.clone(), &own), ERROR_NOT_EMPTY_ARGS);
            refuse(&mut context, funded(&env, [], [Cell { type_: Some(own.clone()), ..env.plain(0) }.sized()]), ERROR_NOT_EMPTY_ARGS);
            refuse(&mut context, funded(&env, [In::new(plain(&own, 0).sized())], []), ERROR_NOT_EMPTY_ARGS);
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
        // The order before the fill and at the price, as (unused CKB, UDT), then whether the fill
        // is too small.
        let (before, at_price, too_small) = if ckb_to_udt {
            // The taker takes `taken` shannons and pays UDT.
            let unused_in = edge_64(&mut rng) % LIMIT + 1;
            let taken = (edge_64(&mut rng) % unused_in).max(1);
            let price = (u128::from(taken) * u128::from(ckb_mul)).div_ceil(u128::from(udt_mul));
            let udt_in = edge_128(&mut rng, u128::MAX - price - 1);
            let partial = unused_in != taken;
            ((unused_in, udt_in), (unused_in - taken, udt_in + price), partial && u128::from(taken) < min_match)
        } else {
            // The taker takes `taken` UDT and pays shannons.
            let taken = edge_128(&mut rng, u128::MAX / u128::from(udt_mul)).max(1);
            let worth = taken * u128::from(udt_mul);
            let price = worth.div_ceil(u128::from(ckb_mul));
            if price >= u128::from(LIMIT) {
                continue;
            }
            let udt_left = edge_128(&mut rng, u128::MAX - taken);
            // The order still holds at most LIMIT once paid one unit more than the price.
            let unused_in = edge_64(&mut rng) % (LIMIT - price as u64);
            ((unused_in, udt_left + taken), (unused_in + price as u64, udt_left), udt_left != 0 && worth < min_match * u128::from(ckb_mul))
        };
        cases += 1;
        let (c2u, u2c) = if ckb_to_udt { ((ckb_mul, udt_mul), (0, 0)) } else { ((0, 0), (ckb_mul, udt_mul)) };
        let order = |(unused, udt): (u64, u128)| {
            let data = order_data_custom(udt, 1, [0x77; 32], 0u32.to_le_bytes(), c2u, u2c, log).to_vec();
            Cell { capacity: deposit_capacity(&env.limit_order, &env.helper, 89, unused), lock: env.limit_order.clone(), type_: Some(env.helper.clone()), data }
        };
        // One unit less than the price, the price, one unit more.
        for paid in [-1i8, 0, 1] {
            let after = if ckb_to_udt {
                (at_price.0, at_price.1.wrapping_add_signed(paid.into()))
            } else {
                (at_price.0.wrapping_add_signed(paid.into()), at_price.1)
            };
            // Funding covers what the taker pays in CKB, so the total capacity stays within 64 bits.
            let funding = plain(&env.funding, after.0.saturating_sub(before.0) + 1_000 * CKB);
            let mut spec = TxSpec {
                inputs: vec![In::new(order(before)), In::new(funding)],
                outputs: vec![Out::from(order(after)), Out { cell: plain(&env.funding, 0), change: true }],
                header_deps: vec![],
            };
            assert!(spec.balance());
            let result = spec.verify(&mut context);
            let case = format!("{}: in {before:?} out {after:?} ratio ({ckb_mul}, {udt_mul}) log {log}", if ckb_to_udt { "CKB to UDT" } else { "UDT to CKB" });
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
/// the scripts and searches for such pairs. With `check` it confirms each row takes the branch named.
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

// The quotient the script computes on RISC-V equals the one this test computes natively. One unit
// more is refused. Both hold on every branch of the division, the two correction steps included.
#[test]
fn conversion_division_is_exact_on_every_branch() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    for (_, amount, ar) in DIVISION_ROWS {
        let value = soft_capped_ickb(amount, ar);
        commit(&mut context, redeem(&env, (1, amount), ar, &[], &[value]));
        refuse(&mut context, redeem(&env, (1, amount), ar, &[], &[value + 1]), ERROR_AMOUNT_MISMATCH);
    }
}

/// The default node limit on the cycles of a transaction it takes into its pool.
const POOL_CYCLES: u64 = 70_000_000;

// `Limit Order` keeps about 8 KB per map node and has 516 KB of heap. Masters met in sorted order
// leave its map nodes half empty. With 383 sorted orders the heap runs out and the script stops.
// That bounds a batch, not an order: each order still mints, fills and melts in a smaller
// transaction. 382 stay far below the block size and the pool cycles. Scattered masters fit more.
#[test]
fn sorted_orders_fit_one_transaction_up_to_382() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let n = 382;
    for spec in [mint(&env, n), fill(&env, n), melt(&env, 0..n)] {
        assert!(commit(&mut context, spec) < POOL_CYCLES);
    }
    for spec in [mint(&env, n + 1), fill(&env, n + 1), melt(&env, 0..n + 1)] {
        refuse(&mut context, spec, ERROR_SCRIPT_PANIC);
    }
    let [_, _, scattered] = orders(400);
    assert!(commit(&mut context, melt(&env, scattered)) < POOL_CYCLES);
}

// 500 withdrawal requests claimed at once, from one deposit block so one header dep serves them
// all, stay within what a node takes into its pool. `Owned Owner` itself has room for six times
// as many; the cycles of 1,000 claims already exceed the pool limit.
#[test]
fn five_hundred_claims_fit_the_pool_cycle_limit() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let keys: Vec<u32> = (0..500).collect();
    assert!(commit(&mut context, claim(&env, &keys)) < POOL_CYCLES);
}

// `Owned Owner` runs out of heap too, far past any claim a node relays: owner cells pointing at
// 3,064 different places stop the script, where 3,063 get the pairing error they deserve. Both
// refuse the transaction.
#[test]
fn owned_owner_holds_3063_pairing_keys() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owners = |count: i32| funded(&env, [], (0..count).map(|k| env.owner(10_000 + k)));
    refuse(&mut context, owners(3_063), ERROR_OWNED_OWNER_MISMATCH);
    refuse(&mut context, owners(3_064), ERROR_SCRIPT_PANIC);
}

// `iCKB Logic` keeps one small map entry per distinct amount, so its heap outlasts the block
// size: 3,870 receipts of distinct amounts take about a whole block. The receipt check still answers.
#[test]
fn ickb_logic_holds_a_block_of_distinct_receipt_amounts() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // The shortest lock, with no args, so that this many receipts fit a block.
    let bare = env.user.clone().as_builder().args(Bytes::new().pack()).build();
    let receipts = (0..3_870).map(|i| env.receipt(1, MIN_DEPOSIT + i).under(&bare));
    refuse(&mut context, funded(&env, [], receipts), ERROR_RECEIPT_MISMATCH);
}
