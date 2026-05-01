use super::*;

// Cell data and witness encoders.
pub(super) fn receipt_data(quantity: u32, amount: u64) -> Bytes {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&quantity.to_le_bytes());
    data.extend_from_slice(&amount.to_le_bytes());
    data.into()
}

pub(super) fn dao_deposit_data() -> Bytes {
    Bytes::from(vec![0u8; 8])
}

pub(super) fn receipt_data_with_trailing_bytes(quantity: u32, amount: u64, trailing: &[u8]) -> Bytes {
    let mut data = receipt_data(quantity, amount).to_vec();
    data.extend_from_slice(trailing);
    data.into()
}

pub(super) fn truncated_bytes(data: Bytes, len: usize) -> Bytes {
    let mut data = data.to_vec();
    data.truncate(len);
    data.into()
}

pub(super) fn udt_data(amount: u128) -> Bytes {
    amount.to_le_bytes().to_vec().into()
}

pub(super) fn xudt_args(ickb_logic_script: &Script) -> Bytes {
    let mut args = ickb_logic_script.calc_script_hash().as_slice().to_vec();
    args.extend_from_slice(&[0, 0, 0, 128]);
    args.into()
}

pub(super) fn xudt_owner_script_witness(owner_script: Script) -> Bytes {
    // Molecule-encode XudtWitness for WitnessArgs.input_type/output_type.
    let owner_script = owner_script.as_bytes();
    let owner_signature = Bytes::new();
    let extension_scripts: &[u8] = &[];
    let empty_extension_data = [4u8, 0, 0, 0];

    let header_size = 4 * 5;
    let mut total_size = header_size;
    let mut offsets = Vec::with_capacity(4);

    offsets.push(total_size);
    total_size += owner_script.len();
    offsets.push(total_size);
    total_size += owner_signature.len();
    offsets.push(total_size);
    total_size += extension_scripts.len();
    offsets.push(total_size);
    total_size += empty_extension_data.len();

    let mut data = Vec::with_capacity(total_size);
    data.extend_from_slice(&(total_size as u32).to_le_bytes());
    for offset in offsets {
        data.extend_from_slice(&(offset as u32).to_le_bytes());
    }
    data.extend_from_slice(owner_script.as_ref());
    data.extend_from_slice(owner_signature.as_ref());
    data.extend_from_slice(extension_scripts);
    data.extend_from_slice(&empty_extension_data);
    data.into()
}

pub(super) fn witness_with_input_type(input_type: Bytes) -> Bytes {
    WitnessArgs::new_builder()
        .input_type(Some(input_type).pack())
        .build()
        .as_bytes()
}

pub(super) fn witness_with_output_type(output_type: Bytes) -> Bytes {
    WitnessArgs::new_builder()
        .output_type(Some(output_type).pack())
        .build()
        .as_bytes()
}

pub(super) fn header_dep_index_witness(index: u64) -> Bytes {
    witness_with_input_type(Bytes::from(index.to_le_bytes().to_vec()))
}

pub(super) fn empty_witness() -> Bytes {
    WitnessArgs::new_builder().build().as_bytes()
}

pub(super) fn withdrawal_request_data(deposit_block_number: u64) -> Bytes {
    deposit_block_number.to_le_bytes().to_vec().into()
}

pub(super) fn owner_distance_data(distance: i32) -> Bytes {
    Bytes::from(distance.to_le_bytes().to_vec())
}

pub(super) fn owner_distance_data_with_trailing_bytes(distance: i32, trailing: &[u8]) -> Bytes {
    let mut data = owner_distance_data(distance).to_vec();
    data.extend_from_slice(trailing);
    data.into()
}

// Limit order data builders.
pub(super) fn order_data_mint(udt_amount: u128, master_distance: i32, ckb_to_udt: (u64, u64)) -> Bytes {
    order_data_custom(
        udt_amount,
        0,
        [0u8; 32],
        master_distance.to_le_bytes(),
        ckb_to_udt,
        (0, 0),
        0,
    )
}

pub(super) fn order_data_match(udt_amount: u128, master_out_point: &OutPoint, ckb_to_udt: (u64, u64)) -> Bytes {
    let index: u32 = master_out_point.index().unpack();
    order_data_custom(
        udt_amount,
        1,
        master_out_point.tx_hash().as_slice().try_into().expect("tx hash"),
        index.to_le_bytes(),
        ckb_to_udt,
        (0, 0),
        0,
    )
}

pub(super) fn order_data_custom(
    udt_amount: u128,
    action: u32,
    tx_hash: [u8; 32],
    index_or_distance: [u8; 4],
    ckb_to_udt: (u64, u64),
    udt_to_ckb: (u64, u64),
    ckb_min_match_log: u8,
) -> Bytes {
    let mut data = Vec::with_capacity(89);
    data.extend_from_slice(&udt_amount.to_le_bytes());
    data.extend_from_slice(&action.to_le_bytes());
    data.extend_from_slice(&tx_hash);
    data.extend_from_slice(&index_or_distance);
    data.extend_from_slice(&ckb_to_udt.0.to_le_bytes());
    data.extend_from_slice(&ckb_to_udt.1.to_le_bytes());
    data.extend_from_slice(&udt_to_ckb.0.to_le_bytes());
    data.extend_from_slice(&udt_to_ckb.1.to_le_bytes());
    data.push(ckb_min_match_log);
    data.into()
}
