use super::*;

pub(super) fn build_many_header_phase2_batch(
    input_count: usize,
    tampered_header_index: Option<(usize, u64)>,
) -> (Context, TransactionView) {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let dao = dao_script(&mut context);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let since = 0x2003e8022a0002f3u64;
    let input_capacity = 123_456_780_000u64;

    let mut inputs = Vec::with_capacity(input_count);
    let mut outputs = Vec::with_capacity(input_count);
    let mut outputs_data = Vec::with_capacity(input_count);
    let mut header_deps = Vec::with_capacity(input_count + 1);
    let mut witnesses = Vec::with_capacity(input_count);
    // Header dep 0 is the shared withdraw header; each per-input witness indexes its deposit header.
    header_deps.push(withdraw_header.hash());

    for i in 0..input_count {
        let deposit_number = 1_554u64 + i as u64;
        let deposit_header = gen_header(deposit_number, GENESIS_AR as u64, 35, 1000, 1000);
        let withdrawing_output = CellOutput::new_builder()
            .capacity(input_capacity.pack())
            .lock(owner_lock.clone())
            .type_(Some(dao.clone()).pack())
            .build();
        let withdrawing_input = context.create_cell(
            withdrawing_output.clone(),
            withdrawal_request_data(deposit_number),
        );
        link_cell_to_header(&mut context, &withdrawing_input, &withdraw_header);

        inputs.push(
            CellInput::new_builder()
                .previous_output(withdrawing_input)
                .since(since.pack())
                .build(),
        );
        outputs.push(
            CellOutput::new_builder()
                .capacity(
                    dao_maximum_withdraw_capacity(
                        &withdrawing_output,
                        withdrawal_request_data(deposit_number).len(),
                        GENESIS_AR as u64,
                        SYNTHETIC_WITHDRAW_AR,
                    )
                    .pack(),
                )
                .lock(owner_lock.clone())
                .build(),
        );
        outputs_data.push(Bytes::new());
        header_deps.push(deposit_header.hash());
        context.insert_header(deposit_header);

        let header_index = tampered_header_index
            .filter(|(tampered_input, _)| *tampered_input == i)
            .map(|(_, index)| index)
            .unwrap_or((i + 1) as u64);
        // Witness order must match input order, and the default header index skips the withdraw header at slot 0.
        witnesses.push(header_dep_index_witness(header_index));
    }

    let mut builder = TransactionBuilder::default()
        .inputs(inputs)
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .cell_dep(secp_data_dep);
    for header_dep in header_deps {
        builder = builder.header_dep(header_dep);
    }
    for witness in witnesses {
        // Preserve 1:1 input/witness ordering so replay fixtures match the on-chain builder layout.
        builder = builder.witness(witness.pack());
    }

    let tx = sign_tx(context.complete_tx(builder.build()), &privkey);
    // Returns `(context, signed_tx)`; callers still decide whether to verify or mutate the replay fixture further.
    (context, tx)
}

// Replay fixture parsers and economics helpers.
pub(super) fn u64_from_hex(hex: &str) -> u64 {
    let hex = hex.strip_prefix("0x").expect("hex prefix missing");
    u64::from_str_radix(hex, 16).expect("invalid u64 hex")
}

pub(super) fn bytes_from_hex(hex: &str) -> Bytes {
    let hex = hex.strip_prefix("0x").expect("hex prefix missing");
    hex::decode(hex).expect("invalid bytes hex").into()
}

pub(super) fn byte32_from_hex(hex: &str) -> Byte32 {
    let hex = hex.strip_prefix("0x").expect("hex prefix missing");
    let bytes = hex::decode(hex).expect("invalid byte32 hex");
    Byte32::from_slice(&bytes).expect("byte32 hex has wrong length")
}

pub(super) fn out_point_from_hex(tx_hash: &str, index: u32) -> OutPoint {
    OutPoint::new(byte32_from_hex(tx_hash), index)
}

pub(super) fn rpc_header(number: &str, epoch: &str, dao: &str) -> ckb_testtool::ckb_types::core::HeaderView {
    HeaderBuilder::default()
        .number(u64_from_hex(number).pack())
        .epoch(u64_from_hex(epoch).pack())
        .dao(byte32_from_hex(dao))
        .build()
}

pub(super) fn soft_capped_ickb(amount: u64, accumulated_rate: u64) -> u128 {
    let raw = u128::from(amount) * u128::from(GENESIS_AR) / u128::from(accumulated_rate);
    let soft_cap = u128::from(100_000 * SHANNONS);
    if raw > soft_cap {
        raw - (raw - soft_cap) / 10
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsers_accept_expected_fixture_shapes() {
        assert_eq!(u64_from_hex("0x2a"), 42);
        assert_eq!(bytes_from_hex("0x1234"), Bytes::from(vec![0x12, 0x34]));
        assert_eq!(
            byte32_from_hex(
                "0x1111111111111111111111111111111111111111111111111111111111111111",
            )
            .as_slice(),
            [0x11; 32],
        );
    }

    #[test]
    #[should_panic(expected = "hex prefix")]
    fn hex_parsers_require_prefix() {
        let _ = u64_from_hex("2a");
    }

    #[test]
    #[should_panic]
    fn bytes_from_hex_rejects_odd_length_payloads() {
        let _ = bytes_from_hex("0x123");
    }

    #[test]
    #[should_panic]
    fn byte32_from_hex_rejects_wrong_width_payloads() {
        let _ = byte32_from_hex(
            "0x11111111111111111111111111111111111111111111111111111111111111",
        );
    }

    #[test]
    fn rpc_header_uses_parsed_values() {
        let header = rpc_header(
            "0x2a",
            "0x708057c0024e7",
            "0x2222222222222222222222222222222222222222222222222222222222222222",
        );

        assert_eq!(header.number(), 42);
        assert_eq!(header.epoch().full_value(), 0x708057c0024e7);
        assert_eq!(header.dao().as_slice(), [0x22; 32]);
    }
}
