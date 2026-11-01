use super::*;

// Scenario: the owner input carries a valid distance plus trailing bytes during phase 2.
// Expectation: melt accepts the input because the leading distance still decodes correctly.
#[test]
fn melt_accepts_owner_distance_trailing_bytes_on_input() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut owner_cell = Cell { data: owner_distance_data_with_trailing_bytes(-1, &[0xaa, 0xbb]).to_vec(), ..env.owner(-1).under(&owner_lock) };
    owner_cell.capacity = owner_cell.occupied();
    let mut spec = melt(&env, 7, owner_cell);
    spec.outputs.push(plain(&owner_lock, claim_capacity(&spec.inputs[0].cell)).into());
    spec.verify(&mut context).expect("owned_owner melt accepts trailing bytes after a valid input owner distance");
}

// Scenario: a canonical Owned Owner withdrawal pair is melted with the correct witness and headers.
// Expectation: the normal phase 2 claim succeeds.
#[test]
fn valid_melt_pair_passes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = melt(&env, 6, env.owner(-1).under(&owner_lock));
    spec.outputs.push(plain(&owner_lock, claim_capacity(&spec.inputs[0].cell)).into());
    spec.verify(&mut context).expect("owned_owner should accept a valid melt pair");
}

// Scenario: the witness points to the wrong deposit header dep index.
// Expectation: phase 2 rejects the claim because the owner pair is bound to the wrong header metadata.
#[test]
fn melt_rejects_misbound_deposit_header_index() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = melt(&env, 7, env.owner(-1).under(&owner_lock));
    spec.inputs[0].witness = header_dep_index_witness(0);
    spec.outputs.push(plain(&owner_lock, 123_468_106_670).into());
    spec.fail(&mut context, ERROR_DAO_INVALID_WITHDRAW_BLOCK);
}

// Scenario: the witness input_type is too short to hold a u64 header index.
// Expectation: witness decoding fails before the claim can proceed.
#[test]
fn melt_rejects_short_header_dep_index_witness() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = melt(&env, 8, env.owner(-1).under(&owner_lock));
    spec.inputs[0].witness = witness_with_input_type(truncated_bytes(Bytes::from(1u64.to_le_bytes().to_vec()), 4));
    spec.outputs.push(plain(&owner_lock, 123_468_106_670).into());
    spec.fail(&mut context, -11);
}

// Scenario: the witness places the header index in output_type instead of input_type.
// Expectation: phase 2 rejects the misplaced witness encoding.
#[test]
fn melt_rejects_header_dep_index_witness_in_output_type() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = melt(&env, 81, env.owner(-1).under(&owner_lock));
    spec.inputs[0].witness = witness_with_output_type(Bytes::from(1u64.to_le_bytes().to_vec()));
    spec.outputs.push(plain(&owner_lock, 123_468_106_670).into());
    spec.fail(&mut context, -11);
}

// Scenario: the owner input distance is truncated during phase 2.
// Expectation: melt rejects the malformed owner cell as an encoding error.
#[test]
fn melt_rejects_truncated_owner_distance_on_input() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let owner_cell = Cell { data: truncated_bytes(owner_distance_data(-1), 1).to_vec(), ..env.owner(-1).under(&owner_lock) };
    let mut spec = melt(&env, 9, owner_cell);
    spec.outputs.push(plain(&owner_lock, 123_468_106_670).into());
    spec.fail(&mut context, ERROR_ENCODING);
}

// Scenario: two separate Owned Owner pairs are consumed with the owner cells deliberately swapped.
// Expectation: phase 2 rejects the cross-pair spend as a mismatch.
#[test]
fn pairs_cannot_be_swapped_during_melt() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    TxSpec {
        inputs: vec![
            In::new(owned(&env, 1000 * CKB, 1554)),
            In::new(env.owner(-1).under(&user2_lock)),
            In::new(owned(&env, 1000 * CKB, 1555)),
            In::new(env.owner(1).under(&user1_lock)),
        ],
        outputs: vec![plain(&user1_lock, 600 * CKB).into(), plain(&user2_lock, 600 * CKB).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}
