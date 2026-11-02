use super::*;

// Owner-distance encoding and melt invariants.

/// The canonical withdrawal with its xUDT input and owner output first, the owner cell holding `owner_data`.
fn owner_first_withdrawal(env: &Env, owner_lock: &Script, owner_data: &[u8]) -> TxSpec {
    let mut spec = withdrawal(env, owner_lock);
    spec.inputs.swap(0, 1);
    spec.outputs.swap(0, 1);
    spec.outputs[0].cell.data = owner_data.to_vec();
    spec
}

// Scenario: the owner distance output is truncated during phase 1 creation.
// Expectation: Owned Owner rejects the malformed distance as an encoding error.
#[test]
fn truncated_owner_distance_output_is_rejected() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    owner_first_withdrawal(&env, &owner_lock, &truncated_bytes(owner_distance_data(1), 1)).fail(&mut context, ERROR_ENCODING);
}

// Scenario: the owner distance output is empty instead of four bytes.
// Expectation: phase 1 rejects the empty distance as an encoding error.
#[test]
fn zero_length_owner_distance_output_is_rejected_as_encoding() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    owner_first_withdrawal(&env, &owner_lock, &[]).fail(&mut context, ERROR_ENCODING);
}

// Scenario: the owner distance has valid leading bytes plus extra trailing data.
// Expectation: phase 1 ignores the trailing bytes and accepts the pair.
#[test]
fn owner_distance_trailing_bytes_are_ignored() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let mut spec = owner_first_withdrawal(&env, &owner_lock, &[1, 0, 0, 0, 0xaa, 0xbb]);
    spec.outputs[0].cell.capacity = spec.outputs[0].cell.occupied();
    spec.verify(&mut context).expect("owned_owner should ignore trailing bytes in owner distance data");
}
