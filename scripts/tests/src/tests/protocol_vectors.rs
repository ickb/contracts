use super::*;

// The protocol vectors compute their expected values with verbatim copies of contract functions
// (scripts/vector-gen). Replaying the rows here checks those values against the release binaries,
// including the order parsing and glue the copies leave out.
#[allow(dead_code)]
#[path = "../../../vector-gen/src/vectors.rs"]
mod vectors;

use vectors::limit_order::Error as Verdict;

#[test]
fn vector_copies_still_match_the_contract_sources() {
    vectors::drift_checks();
}

// Phase 2 of every deposit vector: the receipt mints exactly the expected iCKB, one more or one less fails.
#[test]
fn every_deposit_vector_mints_exactly_its_expected_ickb() {
    for row in vectors::deposit_rows() {
        let mut context = Context::default();
        let funding_lock = always_success_lock(&mut context);
        let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);
        let receipt = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, row.amount);
        let header = insert_header_for_cell(&mut context, &receipt, 0, row.ar);

        for minted in [row.expected_ickb, row.expected_ickb + 1, row.expected_ickb - 1] {
            let tx = TransactionBuilder::default()
                .input(input(receipt.clone()))
                .input(input(funding_cell(&mut context)))
                .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
                .output_data(udt_data(minted).pack())
                .header_dep(header.clone())
                .build();
            let tx = context.complete_tx(tx);
            let result = context.verify(&tx, MAX_CYCLES);
            if minted == row.expected_ickb {
                result.unwrap_or_else(|err| panic!("{}: {err}", row.note));
            } else {
                let err = result.expect_err(&row.note);
                assert_script_error(err, ERROR_AMOUNT_MISMATCH);
            }
        }
    }
}

// Every constructible match vector as a real order continuation: the binary returns the verdict of the copied validate.
#[test]
fn every_limit_order_vector_gets_the_verdict_of_the_copied_validate() {
    let mut unconstructible = Vec::new();
    for row in vectors::limit_rows() {
        let mut context = Context::default();
        let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
        let occupied = deposit_capacity(&limit_order, &helper_type, 89, 0);
        let (in_ckb, in_udt, in_unoccupied) = row.input;
        let (out_ckb, out_udt, out_unoccupied) = row.output;
        // Vectors model a small occupied capacity; a real order cell occupies more. validate only
        // compares input with output, so shifting both by the same occupied capacity keeps the verdict.
        // Input and output share lock, type and data length, so a row whose two sides imply different
        // occupied capacities describes cells consensus cannot create.
        if in_ckb - in_unoccupied != out_ckb - out_unoccupied {
            unconstructible.push(row.note);
            continue;
        }
        let order = |unoccupied: u64, udt: u128| {
            (
                cell(occupied + unoccupied, &limit_order, Some(&helper_type)),
                order_data_custom(
                    udt,
                    1,
                    [0u8; 32],
                    5u32.to_le_bytes(),
                    row.ckb_to_udt.unwrap_or((0, 0)),
                    row.udt_to_ckb.unwrap_or((0, 0)),
                    row.ckb_min_match_log,
                ),
            )
        };
        let (input_cell, input_data) = order(in_unoccupied, in_udt);
        let (output_cell, output_data) = order(out_unoccupied, out_udt);
        let order_input = context.create_cell(input_cell, input_data);

        // The matcher funds any CKB the order gains; the extra 100 CKB covers the funding cell's own
        // 61 CKB of occupied capacity and leaves the rest as fee.
        let funding = funding_cell_of(&mut context, out_unoccupied.saturating_sub(in_unoccupied) + 100 * CKB);
        let tx = TransactionBuilder::default()
            .input(input(order_input))
            .input(input(funding))
            .output(output_cell)
            .output_data(output_data.pack())
            .build();
        let tx = context.complete_tx(tx);
        let result = context.verify(&tx, MAX_CYCLES);
        let expected = match row.verdict {
            Ok(()) => {
                result.unwrap_or_else(|err| panic!("{}: {err}", row.note));
                continue;
            }
            Err(Verdict::DifferentInfo) => ERROR_LIMIT_ORDER_DIFFERENT_INFO,
            Err(Verdict::InvalidMatch) => ERROR_LIMIT_ORDER_INVALID_MATCH,
            Err(Verdict::DecreasingValue) => ERROR_LIMIT_ORDER_DECREASING_VALUE,
            Err(Verdict::AttemptToChangeFulfilled) => ERROR_LIMIT_ORDER_ATTEMPT_TO_CHANGE_FULFILLED,
            Err(Verdict::InsufficientMatch) => ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH,
        };
        let err = result.expect_err(row.note);
        assert_script_error(err, expected);
    }
    // Both shrink a fulfilled CKB->UDT order below its occupied capacity, the only way to reach
    // AttemptToChangeFulfilled on that side: on chain that error is unreachable.
    assert_eq!(
        unconstructible,
        [
            "ckb2udt: value-preserving change of fulfilled order rejected",
            "check order: DecreasingValue precedes AttemptToChangeFulfilled",
        ]
    );
}
