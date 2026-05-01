use super::*;

const RECEIPT_DATA_LEN: usize = 12;
const ORDER_DATA_LEN: usize = 89;

// Little-endian readers make the encoding assertions explicit.
fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("u32 bytes"))
}

fn read_i32_le(bytes: &[u8]) -> i32 {
    i32::from_le_bytes(bytes.try_into().expect("i32 bytes"))
}

fn read_u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("u64 bytes"))
}

fn read_u128_le(bytes: &[u8]) -> u128 {
    u128::from_le_bytes(bytes.try_into().expect("u128 bytes"))
}

// These tests verify the shared data builders used by the larger suites.
#[test]
fn receipt_data_encodes_quantity_then_amount_in_little_endian() {
    let quantity = 0x0102_0304;
    let amount = 0x0102_0304_0506_0708;
    let data = receipt_data(quantity, amount);

    assert_eq!(data.len(), RECEIPT_DATA_LEN);
    assert_eq!(read_u32_le(&data[..4]), quantity);
    assert_eq!(read_u64_le(&data[4..]), amount);
}

#[test]
fn receipt_data_with_trailing_bytes_preserves_the_receipt_prefix() {
    let trailing = [0xaa, 0xbb, 0xcc];
    let prefix = receipt_data(3, 42);
    let data = receipt_data_with_trailing_bytes(3, 42, &trailing);

    assert_eq!(data.len(), RECEIPT_DATA_LEN + trailing.len());
    assert_eq!(&data[..RECEIPT_DATA_LEN], prefix.as_ref());
    assert_eq!(&data[RECEIPT_DATA_LEN..], trailing);
}

#[test]
fn truncated_bytes_keeps_only_the_requested_prefix() {
    let data = Bytes::from(vec![1, 2, 3, 4, 5]);

    assert_eq!(truncated_bytes(data, 3), Bytes::from(vec![1, 2, 3]));
}

#[test]
fn udt_data_encodes_a_full_u128_in_little_endian() {
    let amount = 0x0102_0304_0506_0708_1112_1314_1516_1718u128;
    let data = udt_data(amount);

    assert_eq!(data.len(), 16);
    assert_eq!(read_u128_le(&data), amount);
}

#[test]
fn xudt_args_append_the_owner_mode_flags_after_the_script_hash() {
    let mut context = Context::default();
    let ickb_logic = ickb_logic_script(&mut context);
    let args = xudt_args(&ickb_logic);

    assert_eq!(args.len(), 36);
    assert_eq!(&args[..32], ickb_logic.calc_script_hash().as_slice());
    assert_eq!(&args[32..], [0, 0, 0, 128]);
}

#[test]
fn order_data_mint_uses_zeroed_master_tx_hash_and_signed_distance() {
    let data = order_data_mint(9, -5, (2, 3));

    assert_eq!(data.len(), ORDER_DATA_LEN);
    assert_eq!(read_u128_le(&data[..16]), 9);
    assert_eq!(read_u32_le(&data[16..20]), 0);
    assert_eq!(&data[20..52], [0u8; 32]);
    assert_eq!(read_i32_le(&data[52..56]), -5);
    assert_eq!(read_u64_le(&data[56..64]), 2);
    assert_eq!(read_u64_le(&data[64..72]), 3);
    assert_eq!(read_u64_le(&data[72..80]), 0);
    assert_eq!(read_u64_le(&data[80..88]), 0);
    assert_eq!(data[88], 0);
}

#[test]
fn order_data_match_embeds_the_master_out_point() {
    let tx_hash = Byte32::from_slice(&[7u8; 32]).expect("byte32");
    let out_point = OutPoint::new(tx_hash.clone(), 11);
    let data = order_data_match(13, &out_point, (5, 8));

    assert_eq!(data.len(), ORDER_DATA_LEN);
    assert_eq!(read_u128_le(&data[..16]), 13);
    assert_eq!(read_u32_le(&data[16..20]), 1);
    assert_eq!(&data[20..52], tx_hash.as_slice());
    assert_eq!(read_u32_le(&data[52..56]), 11);
    assert_eq!(read_u64_le(&data[56..64]), 5);
    assert_eq!(read_u64_le(&data[64..72]), 8);
    assert_eq!(read_u64_le(&data[72..80]), 0);
    assert_eq!(read_u64_le(&data[80..88]), 0);
    assert_eq!(data[88], 0);
}

#[test]
fn order_data_custom_preserves_each_field_in_order() {
    let tx_hash = [0x5au8; 32];
    let index_or_distance = [0x11, 0x22, 0x33, 0x44];
    let data = order_data_custom(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        2,
        tx_hash,
        index_or_distance,
        (3, 5),
        (7, 11),
        13,
    );

    assert_eq!(data.len(), ORDER_DATA_LEN);
    assert_eq!(read_u128_le(&data[..16]), 0x0102_0304_0506_0708_1112_1314_1516_1718);
    assert_eq!(read_u32_le(&data[16..20]), 2);
    assert_eq!(&data[20..52], tx_hash);
    assert_eq!(&data[52..56], index_or_distance);
    assert_eq!(read_u64_le(&data[56..64]), 3);
    assert_eq!(read_u64_le(&data[64..72]), 5);
    assert_eq!(read_u64_le(&data[72..80]), 7);
    assert_eq!(read_u64_le(&data[80..88]), 11);
    assert_eq!(data[88], 13);
}

#[test]
fn xudt_owner_script_witness_uses_expected_table_layout() {
    let owner_script = Script::new_builder()
        .code_hash(Byte32::from_slice(&[0x42; 32]).expect("byte32"))
        .hash_type(ScriptHashType::Type.into())
        .args(Bytes::from(vec![1, 2, 3, 4]).pack())
        .build();
    let owner_script_bytes = owner_script.as_bytes();
    let witness = xudt_owner_script_witness(owner_script);

    assert_eq!(read_u32_le(&witness[..4]) as usize, witness.len());
    assert_eq!(read_u32_le(&witness[4..8]), 20);
    assert_eq!(read_u32_le(&witness[8..12]) as usize, 20 + owner_script_bytes.len());
    assert_eq!(read_u32_le(&witness[12..16]), read_u32_le(&witness[8..12]));
    assert_eq!(read_u32_le(&witness[16..20]), read_u32_le(&witness[12..16]));
    assert_eq!(&witness[20..20 + owner_script_bytes.len()], owner_script_bytes.as_ref());
    assert_eq!(&witness[witness.len() - 4..], [4u8, 0, 0, 0]);
}

#[test]
fn witness_with_input_type_sets_only_input_type_bytes() {
    let input_type = Bytes::from(vec![9, 8, 7]);
    let witness = WitnessArgs::from_slice(witness_with_input_type(input_type.clone()).as_ref()).expect("witness args");

    let encoded_input_type: Option<Bytes> = witness.input_type().to_opt().map(|bytes| bytes.raw_data());
    let encoded_output_type: Option<Bytes> = witness.output_type().to_opt().map(|bytes| bytes.raw_data());

    assert_eq!(encoded_input_type, Some(input_type));
    assert_eq!(encoded_output_type, None);
    assert!(witness.lock().to_opt().is_none());
}

#[test]
fn witness_with_output_type_sets_only_output_type_bytes() {
    let output_type = Bytes::from(vec![6, 5, 4]);
    let witness = WitnessArgs::from_slice(witness_with_output_type(output_type.clone()).as_ref()).expect("witness args");

    let encoded_input_type: Option<Bytes> = witness.input_type().to_opt().map(|bytes| bytes.raw_data());
    let encoded_output_type: Option<Bytes> = witness.output_type().to_opt().map(|bytes| bytes.raw_data());

    assert_eq!(encoded_input_type, None);
    assert_eq!(encoded_output_type, Some(output_type));
    assert!(witness.lock().to_opt().is_none());
}

#[test]
fn header_dep_index_witness_encodes_the_index_as_input_type_bytes() {
    let index = 0x0102_0304_0506_0708u64;
    let witness = WitnessArgs::from_slice(header_dep_index_witness(index).as_ref()).expect("witness args");

    let encoded_input_type: Option<Bytes> = witness.input_type().to_opt().map(|bytes| bytes.raw_data());
    let encoded_output_type: Option<Bytes> = witness.output_type().to_opt().map(|bytes| bytes.raw_data());

    assert_eq!(encoded_input_type, Some(Bytes::from(index.to_le_bytes().to_vec())));
    assert_eq!(read_u64_le(encoded_input_type.as_ref().expect("input type bytes").as_ref()), index);
    assert_eq!(encoded_output_type, None);
    assert!(witness.lock().to_opt().is_none());
}

#[test]
fn empty_witness_has_no_lock_or_type_fields() {
    let witness = WitnessArgs::from_slice(empty_witness().as_ref()).expect("witness args");

    assert!(witness.lock().to_opt().is_none());
    assert!(witness.input_type().to_opt().is_none());
    assert!(witness.output_type().to_opt().is_none());
}

#[test]
fn owner_distance_data_with_trailing_bytes_preserves_the_distance_prefix() {
    let distance = -0x0102_0304;
    let trailing = [0xde, 0xad, 0xbe, 0xef];
    let prefix = owner_distance_data(distance);
    let data = owner_distance_data_with_trailing_bytes(distance, &trailing);

    assert_eq!(data.len(), 4 + trailing.len());
    assert_eq!(&data[..4], prefix.as_ref());
    assert_eq!(read_i32_le(&data[..4]), distance);
    assert_eq!(&data[4..], trailing);
}
