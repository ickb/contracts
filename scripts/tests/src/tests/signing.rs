use super::*;

// Signature helpers for whole-transaction binding tests.
pub(super) fn blake160(data: &[u8]) -> Bytes {
    ckb_testtool::ckb_hash::blake2b_256(data)[..20].to_vec().into()
}

pub(super) fn sign_tx(tx: TransactionView, key: &Privkey) -> TransactionView {
    let witnesses_len = tx.witnesses().len();
    sign_tx_by_input_group(tx, key, 0, witnesses_len)
}

pub(super) fn sign_tx_by_input_group(tx: TransactionView, key: &Privkey, begin_index: usize, len: usize) -> TransactionView {
    let tx_hash = tx.hash();
    let mut signed_witnesses: Vec<ckb_testtool::ckb_types::packed::Bytes> = tx
        .inputs()
        .into_iter()
        .enumerate()
        .map(|(i, _)| {
            if i == begin_index {
                let mut blake2b = ckb_testtool::ckb_hash::new_blake2b();
                let mut message = [0u8; 32];
                blake2b.update(&tx_hash.raw_data());

                let witness = WitnessArgs::new_unchecked(tx.witnesses().get(i).unwrap().unpack());
                let zero_lock: Bytes = vec![0u8; SIGNATURE_SIZE].into();
                let witness_for_digest = witness.clone().as_builder().lock(Some(zero_lock).pack()).build();
                let witness_len = witness_for_digest.as_bytes().len() as u64;
                blake2b.update(&witness_len.to_le_bytes());
                blake2b.update(&witness_for_digest.as_bytes());
                ((i + 1)..(i + len)).for_each(|n| {
                    let witness = tx.witnesses().get(n).unwrap();
                    let witness_len = witness.raw_data().len() as u64;
                    blake2b.update(&witness_len.to_le_bytes());
                    blake2b.update(&witness.raw_data());
                });
                blake2b.finalize(&mut message);
                let message = ckb_testtool::ckb_types::H256::from(message);
                let sig = key.sign_recoverable(&message).expect("sign");
                witness
                    .as_builder()
                    .lock(Some(Bytes::from(sig.serialize().to_vec())).pack())
                    .build()
                    .as_bytes()
                    .pack()
            } else {
                tx.witnesses().get(i).unwrap_or_default()
            }
        })
        .collect();
    for i in signed_witnesses.len()..tx.witnesses().len() {
        signed_witnesses.push(tx.witnesses().get(i).unwrap());
    }
    tx.as_advanced_builder().set_witnesses(signed_witnesses).build()
}

pub(super) fn secp_lock(context: &mut Context) -> (Privkey, Script, CellDep) {
    let sighash_out_point = bundled_cell(context, "specs/cells/secp256k1_blake160_sighash_all");
    let secp_data_out_point = bundled_cell(context, "specs/cells/secp256k1_data");
    let privkey = Generator::random_privkey();
    let pubkey = privkey.pubkey().expect("pubkey");
    let args = blake160(&pubkey.serialize());
    let lock = context
        .build_script_with_hash_type(&sighash_out_point, ScriptHashType::Data, args)
        .expect("secp lock");
    let secp_data_dep = CellDep::new_builder()
        .out_point(secp_data_out_point)
        .dep_type(ckb_testtool::ckb_types::core::DepType::Code.into())
        .build();
    (privkey, lock, secp_data_dep)
}

#[test]
fn sign_tx_by_input_group_only_updates_the_target_group_lock_witness() {
    let build_tx = |target_lock: &[u8], grouped_extra_lock: &[u8], unrelated_witness: &[u8]| {
        let inputs = (0..3)
            .map(|index| {
                CellInput::new_builder()
                    .previous_output(
                        OutPoint::new_builder()
                            .tx_hash(Byte32::from_slice(&[index as u8; 32]).expect("tx hash"))
                            .index((index as u32).pack())
                            .build(),
                    )
                    .build()
            })
            .collect::<Vec<_>>();
        let witnesses = vec![
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from_static(b"input-0")).pack())
                .build()
                .as_bytes()
                .pack(),
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(target_lock.to_vec())).pack())
                .build()
                .as_bytes()
                .pack(),
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(grouped_extra_lock.to_vec())).pack())
                .build()
                .as_bytes()
                .pack(),
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(unrelated_witness.to_vec())).pack())
                .build()
                .as_bytes()
                .pack(),
        ];

        TransactionBuilder::default().inputs(inputs).witnesses(witnesses).build()
    };

    let key = Generator::random_privkey();
    let original_tx = build_tx(b"group-target", b"group-extra", b"unrelated");
    let signed_tx = sign_tx_by_input_group(original_tx.clone(), &key, 1, 2);

    assert_eq!(signed_tx.witnesses().len(), original_tx.witnesses().len());
    assert_eq!(signed_tx.witnesses().get(0), original_tx.witnesses().get(0));
    assert_eq!(signed_tx.witnesses().get(2), original_tx.witnesses().get(2));
    assert_eq!(signed_tx.witnesses().get(3), original_tx.witnesses().get(3));

    let original_target = WitnessArgs::new_unchecked(original_tx.witnesses().get(1).unwrap().unpack());
    let signed_target = WitnessArgs::new_unchecked(signed_tx.witnesses().get(1).unwrap().unpack());
    assert_ne!(signed_tx.witnesses().get(1), original_tx.witnesses().get(1));
    assert_eq!(original_target.input_type(), signed_target.input_type());
    assert_eq!(original_target.output_type(), signed_target.output_type());
    assert_eq!(signed_target.lock().to_opt().expect("signed lock").raw_data().len(), SIGNATURE_SIZE);

    let changed_group_tx = sign_tx_by_input_group(build_tx(b"group-target", b"group-extra-updated", b"unrelated"), &key, 1, 2);
    let changed_unrelated_tx = sign_tx_by_input_group(build_tx(b"group-target", b"group-extra", b"unrelated-updated"), &key, 1, 2);

    assert_ne!(changed_group_tx.witnesses().get(1), signed_tx.witnesses().get(1));
    assert_eq!(changed_unrelated_tx.witnesses().get(1), signed_tx.witnesses().get(1));
}

#[test]
fn sign_tx_binds_the_full_witness_set() {
    let build_tx = |trailing_lock: &[u8]| {
        let inputs = (0..2)
            .map(|index| {
                CellInput::new_builder()
                    .previous_output(
                        OutPoint::new_builder()
                            .tx_hash(Byte32::from_slice(&[index as u8; 32]).expect("tx hash"))
                            .index((index as u32).pack())
                            .build(),
                    )
                    .build()
            })
            .collect::<Vec<_>>();
        let witnesses = vec![
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from_static(b"input-0")).pack())
                .build()
                .as_bytes()
                .pack(),
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from_static(b"input-1")).pack())
                .build()
                .as_bytes()
                .pack(),
            WitnessArgs::new_builder()
                .lock(Some(Bytes::from(trailing_lock.to_vec())).pack())
                .build()
                .as_bytes()
                .pack(),
        ];

        TransactionBuilder::default().inputs(inputs).witnesses(witnesses).build()
    };

    let key = Generator::random_privkey();
    let base_tx = build_tx(b"trailing-witness");
    let signed_tx = sign_tx(base_tx.clone(), &key);
    let signed_tx_again = sign_tx(base_tx, &key);
    let changed_trailing_tx = sign_tx(build_tx(b"trailing-witness-updated"), &key);

    assert_eq!(signed_tx.witnesses().len(), 3);
    assert_eq!(signed_tx.witnesses().get(1), signed_tx_again.witnesses().get(1));
    assert_eq!(signed_tx.witnesses().get(2), signed_tx_again.witnesses().get(2));
    assert_eq!(signed_tx.witnesses().get(0), signed_tx_again.witnesses().get(0));
    assert_ne!(changed_trailing_tx.witnesses().get(0), signed_tx.witnesses().get(0));
    assert_eq!(changed_trailing_tx.witnesses().get(1), signed_tx.witnesses().get(1));
}
