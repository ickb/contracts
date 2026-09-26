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
use ckb_testtool::context::Context;

mod encoders;
mod fixtures;
mod helpers;
mod ickb_logic;
mod limit_order;
mod owned_owner;
mod protocol_vectors;
mod replay;
mod replay_helpers;
mod signing;

use encoders::*;
use fixtures::*;
use replay_helpers::*;
use signing::*;

// ckb-testtool runs scripts only, so a fixture could build a cell consensus rejects and still pass.
// Tests verify through this check, which first holds every input and output to its occupied capacity.
trait VerifyRealistic {
    fn verify(&self, tx: &TransactionView, max_cycles: u64) -> Result<u64, Error>;
}

impl VerifyRealistic for Context {
    fn verify(&self, tx: &TransactionView, max_cycles: u64) -> Result<u64, Error> {
        let check = |side: &str, index: usize, cell: &CellOutput, data_len: usize| {
            let occupied = cell
                .occupied_capacity(ckb_testtool::ckb_types::core::Capacity::bytes(data_len).expect("data length"))
                .expect("occupied capacity")
                .as_u64();
            let capacity: u64 = cell.capacity().unpack();
            assert!(
                capacity >= occupied,
                "{side} {index} holds {capacity} shannons, below its occupied capacity {occupied}"
            );
        };
        for (index, input) in tx.inputs().into_iter().enumerate() {
            if let Some((cell, data)) = self.get_cell(&input.previous_output()) {
                check("input", index, &cell, data.len());
            }
        }
        for (index, (cell, data)) in tx.outputs().into_iter().zip(tx.outputs_data()).enumerate() {
            check("output", index, &cell, data.raw_data().len());
        }
        let cycles = self.verify_tx(tx, max_cycles)?;
        // A transaction the scripts accept must also pass the node's transaction checks that
        // ckb-testtool skips: CapacityVerifier and DaoScriptSizeVerifier.
        let dao_code_hash = build_genesis_type_id_script(OUTPUT_INDEX_DAO).calc_script_hash();
        let is_dao = |cell: &CellOutput| {
            cell.type_().to_opt().map_or(false, |script| {
                script.code_hash() == dao_code_hash && script.hash_type() == ScriptHashType::Type.into()
            })
        };
        let inputs: Vec<_> = tx.inputs().into_iter().map(|input| (input.previous_output(), self.get_cell(&input.previous_output()))).collect();
        // Outputs may not exceed inputs, except in a transaction with a DAO input, whose
        // compensation the DAO script checks instead.
        if !inputs.iter().any(|(_, cell)| cell.as_ref().map_or(false, |(cell, _)| is_dao(cell))) {
            let inputs_sum: u64 = inputs.iter().filter_map(|(_, cell)| cell.as_ref()).map(|(cell, _)| Unpack::<u64>::unpack(&cell.capacity())).sum();
            let outputs_sum: u64 = tx.outputs().into_iter().map(|cell| Unpack::<u64>::unpack(&cell.capacity())).sum();
            assert!(outputs_sum <= inputs_sum, "outputs hold {outputs_sum} shannons, more than the inputs' {inputs_sum}");
        }
        // A deposit committed since the node's starting block and the withdrawal request at its
        // index use locks of the same serialized size; older deposits are exempt.
        for (index, ((out_point, deposit), output)) in inputs.iter().zip(tx.outputs()).enumerate() {
            let Some((deposit, data)) = deposit else {
                continue;
            };
            let exempt = self
                .transaction_infos
                .get(out_point)
                .map_or(false, |info| info.block_number < DAO_LOCK_SIZE_RULE_START_BLOCK);
            if !exempt && is_dao(deposit) && is_dao(&output) && data.iter().all(|byte| *byte == 0) {
                assert_eq!(
                    deposit.lock().total_size(),
                    output.lock().total_size(),
                    "DAO output {index} changes the lock size of the deposit it withdraws"
                );
            }
        }
        Ok(cycles)
    }
}

#[test]
#[should_panic(expected = "below its occupied capacity")]
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
#[should_panic(expected = "changes the lock size of the deposit it withdraws")]
fn verify_rejects_a_withdrawal_request_whose_lock_size_differs() {
    verify_withdrawal_with_smaller_request_lock(DAO_LOCK_SIZE_RULE_START_BLOCK);
}

#[test]
fn verify_exempts_deposits_older_than_the_lock_size_rule() {
    verify_withdrawal_with_smaller_request_lock(DAO_LOCK_SIZE_RULE_START_BLOCK - 1);
}

// Shared test constants and on-chain error codes.
const MAX_CYCLES: u64 = 10_000_000;
// The node's starting_block_limiting_dao_withdrawing_lock (ckb spec/src/consensus.rs).
const DAO_LOCK_SIZE_RULE_START_BLOCK: u64 = 10_000_000;
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
const ERROR_DAO_INCORRECT_CAPACITY: i8 = -15;
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
