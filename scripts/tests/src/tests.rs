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
mod replay;
mod replay_helpers;
mod signing;

use encoders::*;
use fixtures::*;
use replay_helpers::*;
use signing::*;

// Shared test constants and on-chain error codes.
const MAX_CYCLES: u64 = 10_000_000;
const SHANNONS: u64 = 100_000_000;
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
const ERROR_AMOUNT_MISMATCH: i8 = 11;
const ERROR_AMOUNT_UNREASONABLY_BIG: i8 = 12;
const ERROR_LIMIT_ORDER_INVALID_CONFIGURATION: i8 = 21;
const ERROR_LIMIT_ORDER_DIFFERENT_INFO: i8 = 16;
const ERROR_DAO_TOO_MANY_OUTPUT_CELLS: i8 = -18;
const ERROR_LIMIT_ORDER_SAME_MASTER: i8 = 14;
const ERROR_LIMIT_ORDER_SCRIPT_MISUSE: i8 = 15;
const ERROR_LIMIT_ORDER_INVALID_ACTION: i8 = 7;
const ERROR_LIMIT_ORDER_NON_ZERO_PADDING: i8 = 8;
const ERROR_LIMIT_ORDER_INVALID_RATIO: i8 = 9;
const ERROR_LIMIT_ORDER_INVALID_CKB_MIN_MATCH_LOG: i8 = 10;
const ERROR_LIMIT_ORDER_CONCAVE_RATIO: i8 = 11;
const ERROR_LIMIT_ORDER_BOTH_RATIOS_NULL: i8 = 12;
const ERROR_LIMIT_ORDER_INVALID_MATCH: i8 = 17;
const ERROR_LIMIT_ORDER_DECREASING_VALUE: i8 = 18;
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
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(ickb_logic.clone())
            .build(),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(ickb_logic.clone())
                .build(),
            CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(ickb_logic)
                .build(),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();
    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);

    let mut context = Context::default();
    let ickb_logic = ickb_logic_script(&mut context);
    let input_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(ickb_logic.clone())
            .build(),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(ickb_logic.clone())
                .build(),
            CellOutput::new_builder()
                .capacity(500u64.pack())
                .lock(ickb_logic)
                .build(),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();
    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SCRIPT_MISUSE);
}
