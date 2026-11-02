use super::*;
use ckb_testtool::builtin::ALWAYS_SUCCESS;
use ckb_testtool::ckb_chain_spec::{build_genesis_type_id_script, OUTPUT_INDEX_DAO};
use ckb_testtool::ckb_crypto::secp::{Generator, Privkey};
use ckb_testtool::ckb_error::Error;
use ckb_testtool::ckb_types::{
    bytes::Bytes,
    core::{EpochExt, HeaderBuilder, ScriptHashType, TransactionBuilder, TransactionView},
    packed::*,
    prelude::*,
};
use ckb_chain_spec::{consensus::Consensus, ChainSpec};
use ckb_dao::DaoCalculator;
use ckb_resource::Resource;
use ckb_testtool::ckb_types::core::cell::{CellMetaBuilder, ResolvedTransaction};
use ckb_testtool::ckb_types::core::Capacity;
use ckb_testtool::context::Context;
use ckb_traits::CellDataProvider;
use ckb_verification::{CapacityVerifier, DaoScriptSizeVerifier, NonContextualTransactionVerifier, Since, SinceMetric};
use std::sync::{Arc, OnceLock};

mod encoders;
mod fixtures;
mod fuzz;
mod helpers;
mod ickb_logic;
mod limit_order;
mod owned_owner;
mod protocol_vectors;
mod replay;
mod replay_helpers;
mod signing;
mod tx;

use encoders::*;
use fixtures::*;
use replay_helpers::*;
use signing::*;
use tx::*;

// ckb-testtool runs scripts only, so a fixture could build a transaction the node rejects and still
// pass. Tests verify through this check, which runs the node's own transaction verifiers on every
// transaction, accepted or rejected, and the invariant oracle on every accepted one.
trait VerifyRealistic {
    fn verify(&self, tx: &TransactionView, max_cycles: u64) -> Result<u64, Error>;
    fn verify_where_node_also_rejects(&self, tx: &TransactionView, max_cycles: u64, node_rule: &str) -> Result<u64, Error>;
    fn node_rejection(&self, tx: &TransactionView) -> Option<String>;
}

/// Mainnet's consensus as the node builds it from its bundled chain spec: block limits, the DAO
/// type hash, the DAO lock-size start block and the active hardforks all come from there.
fn mainnet() -> Arc<Consensus> {
    static MAINNET: OnceLock<Arc<Consensus>> = OnceLock::new();
    Arc::clone(MAINNET.get_or_init(|| {
        let spec = ChainSpec::load_from(&Resource::bundled("specs/mainnet.toml".to_string())).expect("mainnet spec");
        Arc::new(spec.build_consensus().expect("mainnet consensus"))
    }))
}

/// `DaoScriptSizeVerifier` takes its loader by value; this lends it the context.
struct Loaded<'a>(&'a Context);

impl CellDataProvider for Loaded<'_> {
    fn get_cell_data(&self, out_point: &OutPoint) -> Option<Bytes> {
        self.0.get_cell_data(out_point)
    }

    fn get_cell_data_hash(&self, out_point: &OutPoint) -> Option<Byte32> {
        self.0.get_cell_data_hash(out_point)
    }
}

impl VerifyRealistic for Context {
    fn verify(&self, tx: &TransactionView, max_cycles: u64) -> Result<u64, Error> {
        let result = self.verify_tx(tx, max_cycles);
        // A rejection only shows something about the scripts when the node would take the
        // transaction otherwise; a test whose point overlaps a node rule says so through
        // verify_where_node_also_rejects.
        if let Some(rejection) = self.node_rejection(tx) {
            let built = if result.is_ok() { "the scripts accept" } else { "the test builds" };
            panic!("{built} a transaction the node rejects: {rejection}");
        }
        let cycles = result?;
        let violations = fuzz::check_invariants(self, tx);
        assert!(violations.is_empty(), "the scripts accept a transaction that breaks an invariant: {violations:?}");
        Ok(cycles)
    }

    /// Verifies a transaction that also breaks the node rule named by `node_rule`, for tests whose
    /// point overlaps that rule; a different first node rejection still fails the test.
    fn verify_where_node_also_rejects(&self, tx: &TransactionView, max_cycles: u64, node_rule: &str) -> Result<u64, Error> {
        let rejection = self.node_rejection(tx).expect("the node accepts this transaction, so verify it with verify");
        assert!(rejection.contains(node_rule), "the node rejects this transaction for another reason: {rejection}");
        self.verify_tx(tx, max_cycles)
    }

    /// Why the node would reject `tx` before or after its scripts, if it would. Time-dependent
    /// checks (since maturity, cellbase maturity) are left out: waiting satisfies them.
    fn node_rejection(&self, tx: &TransactionView) -> Option<String> {
        let consensus = mainnet();
        let mut inputs = Vec::new();
        for (index, input) in tx.inputs().into_iter().enumerate() {
            let out_point = input.previous_output();
            let Some((cell, data)) = self.cells.get(&out_point) else {
                return Some(format!("input {index} spends no live cell"));
            };
            // An input was an output once, so it holds its occupied capacity.
            if cell.is_lack_of_capacity(Capacity::bytes(data.len()).expect("data length")).expect("occupied capacity") {
                return Some(format!("input {index} holds less than its occupied capacity"));
            }
            let mut meta = CellMetaBuilder::from_cell_output(cell.clone(), data.clone()).out_point(out_point.clone());
            if let Some(info) = self.transaction_infos.get(&out_point) {
                meta = meta.transaction_info(info.clone());
            }
            inputs.push(meta.build());
        }
        // The node's resolver rejects a transaction that spends one cell twice.
        let mut spent: Vec<_> = inputs.iter().map(|meta| meta.out_point.clone()).collect();
        spent.sort_by_key(|out_point| out_point.as_bytes());
        if spent.windows(2).any(|pair| pair[0] == pair[1]) {
            return Some("an input cell is spent twice".to_string());
        }
        // SinceVerifier's checks that no amount of waiting satisfies.
        for (index, input) in tx.inputs().into_iter().enumerate() {
            let since = Since(input.since().into());
            let well_formed = since.0 == 0
                || since.flags_is_valid()
                    && match since.extract_metric() {
                        Some(SinceMetric::EpochNumberWithFraction(epoch)) => epoch.is_well_formed_increment(),
                        Some(_) => true,
                        None => false,
                    };
            if !well_formed {
                return Some(format!("input {index} has an invalid since"));
            }
        }
        let rtx = Arc::new(ResolvedTransaction { transaction: tx.clone(), resolved_cell_deps: Vec::new(), resolved_inputs: inputs, resolved_dep_groups: Vec::new() });
        let checks = [
            NonContextualTransactionVerifier::new(tx, &consensus).verify().err(),
            CapacityVerifier::new(Arc::clone(&rtx), consensus.dao_type_hash()).verify().err(),
            DaoCalculator::new(&consensus, self).transaction_fee(&rtx).err().map(Into::into),
            DaoScriptSizeVerifier::new(Arc::clone(&rtx), Arc::clone(&consensus), Loaded(self)).verify().err(),
        ];
        checks.into_iter().flatten().next().map(|error| format!("{error:?}"))
    }
}

/// A receipt for zero deposits, which iCKB Logic rejects, in a transaction whose output exceeds
/// its input, which the node rejects too.
fn rejected_and_unfunded(context: &mut Context) -> TxSpec {
    let env = Env::new(context);
    TxSpec { inputs: vec![In::new(env.plain(100 * CKB))], outputs: vec![env.receipt(0, 1_000 * CKB).into()], ..Default::default() }
}

#[test]
#[should_panic(expected = "the test builds a transaction the node rejects")]
fn verify_rejects_a_rejection_test_built_on_a_node_invalid_transaction() {
    let mut context = Context::default();
    rejected_and_unfunded(&mut context).fail(&mut context, ERROR_EMPTY_RECEIPT);
}

#[test]
#[should_panic(expected = "the node rejects this transaction for another reason")]
fn verify_where_node_also_rejects_requires_the_named_rule() {
    let mut context = Context::default();
    let tx = rejected_and_unfunded(&mut context).build(&mut context);
    let _ = context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "InvalidDaoFormat");
}

#[test]
#[should_panic(expected = "InsufficientCellCapacity(Outputs[0])")]
fn verify_rejects_a_cell_below_its_occupied_capacity() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let funding = context.create_cell(
        cell(1_000 * CKB, &lock, None),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(input(funding))
        .output(cell(40 * CKB, &lock, None))
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);
    let _ = context.verify(&tx, MAX_CYCLES);
}

/// A stand-in for the NervosDAO type that accepts every transaction, so a test reaches the node
/// checks with shapes the real DAO script would reject first.
fn permissive_dao_script(context: &mut Context) -> Script {
    let lock = always_success_lock(context);
    let type_script = build_genesis_type_id_script(OUTPUT_INDEX_DAO);
    let code = context.create_cell(
        cell(capacity_for_data(ALWAYS_SUCCESS.len() as u64), &lock, Some(&type_script)),
        ALWAYS_SUCCESS.clone(),
    );
    context.build_script(&code, Bytes::new()).expect("dao script")
}

#[test]
#[should_panic(expected = "The DAO output data at index 0 is inconsistent")]
fn verify_rejects_dao_output_data_without_a_deposit_at_its_index() {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let dao = permissive_dao_script(&mut context);
    let funding = context.create_cell(cell(1_000 * CKB, &lock, None), Bytes::new());
    let tx = TransactionBuilder::default()
        .input(input(funding))
        .output(cell(1_000 * CKB, &lock, Some(&dao)))
        .output_data(withdrawal_request_data(1_000).pack())
        .build();
    let tx = context.complete_tx(tx);
    let _ = context.verify(&tx, MAX_CYCLES);
}

/// A phase 2 claim of one withdrawing cell under the stand-in DAO type, taking its maximum
/// withdraw plus `extra` shannons, with or without the withdrawing block in the header deps and
/// the witness header index.
fn verify_claim_with_stand_in_dao(extra: u64, withdrawing_block_listed: bool, witness_index: bool) -> Result<u64, Error> {
    let mut context = Context::default();
    let lock = always_success_lock(&mut context);
    let dao = permissive_dao_script(&mut context);
    let deposit_header = gen_header(1_554, GENESIS_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let request = cell(1_000 * CKB, &lock, Some(&dao));
    let claimed = dao_maximum_withdraw_capacity(&request, 8, GENESIS_AR, SYNTHETIC_WITHDRAW_AR) + extra;
    let request = context.create_cell(request, withdrawal_request_data(1_554));
    link_cell_to_header(&mut context, &request, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let mut builder = TransactionBuilder::default()
        .input(input(request))
        .output(cell(claimed, &lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(deposit_header.hash())
        .witness(if witness_index { header_dep_index_witness(0) } else { Bytes::new() }.pack());
    if withdrawing_block_listed {
        builder = builder.header_dep(withdraw_header.hash());
    }
    let tx = context.complete_tx(builder.build());
    context.verify(&tx, MAX_CYCLES)
}

#[test]
fn verify_accepts_a_claim_of_exactly_its_maximum_withdraw() {
    verify_claim_with_stand_in_dao(0, true, true).expect("the node accepts an exact claim");
}

#[test]
#[should_panic(expected = "kind: Dao, inner: Overflow")]
fn verify_rejects_a_claim_above_its_maximum_withdraw() {
    let _ = verify_claim_with_stand_in_dao(1, true, true);
}

#[test]
#[should_panic(expected = "kind: Dao, inner: InvalidOutPoint")]
fn verify_rejects_a_claim_without_its_withdrawing_block() {
    let _ = verify_claim_with_stand_in_dao(0, false, true);
}

#[test]
#[should_panic(expected = "kind: Dao, inner: InvalidDaoFormat")]
fn verify_rejects_a_claim_without_a_witness_header_index() {
    let _ = verify_claim_with_stand_in_dao(0, true, false);
}

/// A phase 1 withdrawal whose request lock is smaller than its deposit's, for a deposit
/// committed at `block_number`.
fn verify_withdrawal_with_smaller_request_lock(block_number: u64) {
    let mut context = Context::default();
    let deposit_lock = named_always_success_lock(&mut context, b"depositor");
    let request_lock = always_success_lock(&mut context);
    let dao = dao_script(&mut context);
    let capacity = 1_000 * CKB;
    let deposit = create_deposit(&mut context, capacity, &deposit_lock, &dao);
    let header = gen_header(block_number, GENESIS_AR, 35, block_number - 10, 1000);
    link_cell_to_header(&mut context, &deposit, &header);
    let tx = TransactionBuilder::default()
        .input(input(deposit))
        .output(cell(capacity, &request_lock, Some(&dao)))
        .output_data(withdrawal_request_data(block_number).pack())
        .header_dep(header.hash())
        .build();
    let tx = context.complete_tx(tx);
    context.verify(&tx, MAX_CYCLES).expect("the DAO script accepts the withdrawal");
}

#[test]
#[should_panic(expected = "lock script size of deposit cell at index 0 does not match")]
fn verify_rejects_a_withdrawal_request_whose_lock_size_differs() {
    verify_withdrawal_with_smaller_request_lock(mainnet().starting_block_limiting_dao_withdrawing_lock());
}

#[test]
fn verify_exempts_deposits_older_than_the_lock_size_rule() {
    verify_withdrawal_with_smaller_request_lock(mainnet().starting_block_limiting_dao_withdrawing_lock() - 1);
}

// Shared test constants and on-chain error codes.
const MAX_CYCLES: u64 = 10_000_000;
const CKB: u64 = 100_000_000;
const GENESIS_AR: u64 = 10_000_000_000_000_000;
const SIGNATURE_SIZE: usize = 65;
const SYNTHETIC_DEPOSIT_AR: u64 = GENESIS_AR;
const SYNTHETIC_WITHDRAW_AR: u64 = GENESIS_AR + 1_000_000;

const ERROR_ENCODING: i8 = 4;
const ERROR_ITEM_MISSING: i8 = 2;
const ERROR_NOT_EMPTY_ARGS: i8 = 5;
const ERROR_DEPOSIT_TOO_SMALL: i8 = 7;
const ERROR_DEPOSIT_TOO_BIG: i8 = 8;
const ERROR_EMPTY_RECEIPT: i8 = 9;
const ERROR_RECEIPT_MISMATCH: i8 = 10;
const ERROR_SCRIPT_MISUSE: i8 = 6;
const ERROR_LENGTH_NOT_ENOUGH: i8 = 3;
const ERROR_AMOUNT_MISMATCH: i8 = 11;
const ERROR_AMOUNT_UNREASONABLY_BIG: i8 = 12;
const ERROR_DAO_INVALID_WITHDRAW_BLOCK: i8 = -14;
const ERROR_DAO_INVALID_WITHDRAWING_CELL: i8 = -20;
const ERROR_LIMIT_ORDER_INVALID_CONFIGURATION: i8 = 21;
const ERROR_LIMIT_ORDER_DIFFERENT_INFO: i8 = 16;
const ERROR_DAO_TOO_MANY_OUTPUT_CELLS: i8 = -18;
const ERROR_DAO_NEWLY_CREATED_CELL: i8 = -19;
const ERROR_LIMIT_ORDER_SAME_MASTER: i8 = 14;
const ERROR_LIMIT_ORDER_SCRIPT_MISUSE: i8 = 15;
const ERROR_LIMIT_ORDER_INVALID_ACTION: i8 = 7;
const ERROR_LIMIT_ORDER_NON_ZERO_PADDING: i8 = 8;
const ERROR_LIMIT_ORDER_INVALID_RATIO: i8 = 9;
const ERROR_LIMIT_ORDER_INVALID_CKB_MIN_MATCH_LOG: i8 = 10;
const ERROR_LIMIT_ORDER_CONCAVE_RATIO: i8 = 11;
const ERROR_LIMIT_ORDER_BOTH_RATIOS_NULL: i8 = 12;
const ERROR_LIMIT_ORDER_MISSING_UDT_TYPE: i8 = 13;
const ERROR_LIMIT_ORDER_INVALID_MATCH: i8 = 17;
const ERROR_LIMIT_ORDER_DECREASING_VALUE: i8 = 18;
const ERROR_LIMIT_ORDER_ATTEMPT_TO_CHANGE_FULFILLED: i8 = 19;
const ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH: i8 = 20;
const ERROR_SCRIPT_PANIC: i8 = -1;
const ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST: i8 = 6;
const ERROR_OWNED_OWNER_SCRIPT_MISUSE: i8 = 7;
const ERROR_OWNED_OWNER_MISMATCH: i8 = 8;
const ERROR_XUDT_AMOUNT: i8 = -52;
const ERROR_SECP256K1_BLAKE160_SIGHASH_ALL: i8 = -31;

fn assert_script_error(err: Error, err_code: i8) {
    let error_string = err.to_string();
    assert!(
        error_string.contains(format!("error code {err_code} ").as_str()),
        "error_string: {error_string}, expected_error_code: {err_code}"
    );
}

fn fail(context: &Context, tx: &TransactionView, err_code: i8) {
    assert_script_error(context.verify(tx, MAX_CYCLES).unwrap_err(), err_code);
}

fn assert_script_error_in(err: Error, err_codes: &[i8]) {
    let error_string = err.to_string();
    assert!(
        err_codes
            .iter()
            .any(|err_code| error_string.contains(format!("error code {err_code} ").as_str())),
        "error_string: {error_string}, expected_one_of: {err_codes:?}"
    );
}

fn load_binary(name: &str) -> Bytes {
    Loader::default().load_binary(name)
}

#[test]
fn type_role_scan_with_512_unrelated_outputs_stays_within_test_cycle_budget() {
    const PADDING_OUTPUTS: usize = 512;

    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);
    let deposit_amount = 1_000 * CKB;
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let receipt_input = create_receipt(&mut context, &user_lock, &ickb_logic, 1, deposit_amount);
    link_cell_to_header(&mut context, &receipt_input, &deposit_header);
    let funding_input = context.create_cell(
        cell((PADDING_OUTPUTS as u64 + 1) * 100 * CKB, &user_lock, None),
        Bytes::new(),
    );

    let mut outputs = vec![cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt))];
    let mut outputs_data = vec![udt_data(u128::from(deposit_amount))];
    for _ in 0..PADDING_OUTPUTS {
        outputs.push(cell(100 * CKB, &user_lock, None));
        outputs_data.push(Bytes::new());
    }

    let tx = TransactionBuilder::default()
        .input(input(receipt_input))
        .input(input(funding_input))
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .header_dep(deposit_header.hash())
        .build();
    let tx = context.complete_tx(tx);
    // verify fails with ExceededMaximumCycles above the budget, so passing is the bound.
    context
        .verify(&tx, MAX_CYCLES)
        .expect("type-role scan with 512 unrelated outputs should stay within the test cycle budget");
}

// Harness sanity checks.
#[test]
fn release_binary_hashes_match_deployment_references() {
    let ickb_logic_hash = CellOutput::calc_data_hash(&load_binary("ickb_logic"));
    let limit_order_hash = CellOutput::calc_data_hash(&load_binary("limit_order"));
    let owned_owner_hash = CellOutput::calc_data_hash(&load_binary("owned_owner"));

    assert_eq!(
        ickb_logic_hash,
        Byte32::from_slice(
            &hex::decode("2a8100ab5990fa055ab1b50891702e1e895c7bd1df6322cd725c1a6115873bd3")
                .expect("ickb logic deployment hash"),
        )
        .expect("byte32")
    );
    assert_eq!(
        limit_order_hash,
        Byte32::from_slice(
            &hex::decode("49dfb6afee5cc8ac4225aeea8cb8928b150caf3cd92fea33750683c74b13254a")
                .expect("limit order deployment hash"),
        )
        .expect("byte32")
    );
    assert_eq!(
        owned_owner_hash,
        Byte32::from_slice(
            &hex::decode("acc79e07d107831feef4c70c9e683dac5644d5993b9cb106dca6e74baa381bd0")
                .expect("owned owner deployment hash"),
        )
        .expect("byte32")
    );
}

#[test]
fn scaffolding_tests_fail_for_the_reasons_reported() {
    let mut context = Context::default();
    let ickb_logic = data1_script(&mut context, "ickb_logic", Bytes::from(vec![42]));
    let input_out_point = context.create_cell(
        cell(1000 * CKB, &ickb_logic, None),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(input(input_out_point))
        .outputs(vec![
            cell(500 * CKB, &ickb_logic, None),
            cell(500 * CKB, &ickb_logic, None),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();
    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);

    let mut context = Context::default();
    let ickb_logic = ickb_logic_script(&mut context);
    let input_out_point = context.create_cell(
        cell(1000 * CKB, &ickb_logic, None),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(input(input_out_point))
        .outputs(vec![
            cell(500 * CKB, &ickb_logic, None),
            cell(500 * CKB, &ickb_logic, None),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();
    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_SCRIPT_MISUSE);
}
