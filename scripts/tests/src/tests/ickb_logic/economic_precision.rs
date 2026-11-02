use super::*;

// Build phase2 receipt-to-xUDT conversions around a reported rounding edge: overstating the mint by even one shannon fails, while the exact header-normalized amount passes, proving the receipt path uses actual shannon precision rather than a coarser rounded claim.
#[test]
fn reported_rounding_claim_is_blocked_by_actual_shannon_precision() {
    let mut context = Context::default();
    let env = Env::new(&mut context);

    let ar = 11_509_953_685_250_771u64;
    let amount_1152 = 1_152u64 * CKB;
    let amount_1151 = 1_151u64 * CKB;
    let exact_ickb_1152 = u128::from(amount_1152) * u128::from(GENESIS_AR) / u128::from(ar);
    let exact_ickb_1151 = u128::from(amount_1151) * u128::from(GENESIS_AR) / u128::from(ar);

    assert!(exact_ickb_1152 > 1_000 * CKB as u128);
    assert!(exact_ickb_1151 < exact_ickb_1152);

    let receipt_1152 = env.receipt(1, amount_1152);
    let receipt_1152_header = gen_header(1, ar, 1, 1, 1000);
    mint_at(&env, receipt_1152.clone(), &receipt_1152_header, 1_000 * CKB as u128, true).fail(&mut context, ERROR_AMOUNT_MISMATCH);
    mint_at(&env, receipt_1152, &receipt_1152_header, exact_ickb_1152, true)
        .verify(&mut context)
        .expect("1152 CKB receipt should mint its exact shannon-precision amount");

    let receipt_1151_header = gen_header(2, ar, 1, 1, 1000);
    mint_at(&env, env.receipt(1, amount_1151), &receipt_1151_header, exact_ickb_1152, true).fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Pin the shifted soft-cap edge under a non-genesis AR: normalization reaches the cap at 110k CKB. Ten shannons past it, normalizing first mints cap+9 while applying the haircut first would mint cap+8, so the passing cap+9 mint proves normalization precedes the per-deposit haircut; eleven past it, cap+10 must fail.
#[test]
fn non_genesis_ar_soft_cap_boundary_preserves_integer_operation_order() {
    let mut context = Context::default();
    let env = Env::new(&mut context);

    let ar = 11_000_000_000_000_000u64;
    let soft_cap = 100_000 * CKB as u128;
    let shifted_boundary = 110_000 * CKB;
    let normalized = |amount: u64| {
        u128::from(amount) * u128::from(GENESIS_AR) / u128::from(ar)
    };

    assert_eq!(normalized(shifted_boundary), soft_cap);
    assert_eq!(normalized(shifted_boundary + 10), soft_cap + 9);
    assert_eq!(normalized(shifted_boundary + 11), soft_cap + 10);
    assert_eq!(soft_capped_ickb(shifted_boundary + 10, ar), soft_cap + 9);
    assert_eq!(soft_capped_ickb(shifted_boundary + 11, ar), soft_cap + 9);

    let mut assert_mint = |block_number: u64, amount: u64, minted: u128, should_pass: bool| {
        let receipt_header = gen_header(block_number, ar, 1, 1, 1000);
        let result = mint_at(&env, env.receipt(1, amount), &receipt_header, minted, true).verify(&mut context);

        if should_pass {
            result.expect("exact mint at the AR-shifted soft-cap boundary should verify");
        } else {
            assert_script_error(result.unwrap_err(), ERROR_AMOUNT_MISMATCH);
        }
    };

    assert_mint(1, shifted_boundary, soft_cap, true);
    assert_mint(2, shifted_boundary + 10, soft_cap + 9, true);
    assert_mint(3, shifted_boundary + 11, soft_cap + 10, false);
    assert_mint(4, shifted_boundary + 11, soft_cap + 9, true);
}

// Build phase1 deposit-to-withdrawal conversions around the same rounding claim: burning a rounded-down iCKB amount fails, but burning the exact header-normalized amount passes, so withdrawal initiation also enforces shannon-precise value matching.
#[test]
fn reported_rounding_withdrawal_claim_is_blocked_by_actual_shannon_precision() {
    let mut context = Context::default();
    let env = Env::new(&mut context);

    let ar = 11_509_953_685_250_771u64;
    let amount_1152 = 1_152u64 * CKB;
    let amount_1151 = 1_151u64 * CKB;
    let exact_ickb_1152 = u128::from(amount_1152) * u128::from(GENESIS_AR) / u128::from(ar);

    let header = gen_header(1554, ar, 35, 1000, 1000);

    // Withdraw a deposit of `amount` to an owned_owner request, burning `burned` iCKB.
    let withdraw = |amount: u64, burned: u128| {
        let deposit = env.deposit(&env.ickb, amount);
        TxSpec {
            inputs: vec![In::new(deposit.clone()).in_block(&header), In::new(env.udt(burned))],
            outputs: vec![request(&deposit, &env.owned_owner, 1554).into(), env.owner(-1).into()],
            header_deps: vec![header.clone()],
        }
    };

    withdraw(amount_1152, 1_000 * CKB as u128).fail(&mut context, ERROR_AMOUNT_MISMATCH);
    withdraw(amount_1152, exact_ickb_1152)
        .verify(&mut context)
        .expect("withdrawing 1152 CKB should require the exact shannon-precision iCKB amount");
    withdraw(amount_1151, exact_ickb_1152).fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Build full phase1-plus-DAO-phase2 flows for older, newer, and soft-capped deposits: each case should pass and the later normalized claim value must stay within one shannon of the phase1 burn, showing the protocol does not open a profitable precision gap across time or size boundaries.
#[test]
fn withdrawal_burn_matches_later_protocol_value_within_one_shannon_across_header_and_soft_cap_cases() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);

    let withdraw_ar = 12_000_000_000_000_000u64;
    let withdraw_header = gen_header(2_000_610, withdraw_ar, 575, 2_000_000, 1100);

    let mut run_case = |amount: u64, deposit_number: u64, deposit_ar: u64| -> u128 {
        let (burned_ickb, claim_capacity, deposit_occupied_capacity) = burn_and_claim(
            &mut context,
            (&env, &user_lock),
            (&withdraw_header, withdraw_ar),
            (amount, deposit_number, deposit_ar),
        );
        let later_ickb_value =
            soft_capped_ickb(claim_capacity - deposit_occupied_capacity, withdraw_ar);
        assert!(burned_ickb >= later_ickb_value);
        assert!(
            burned_ickb - later_ickb_value <= 1,
            "phase1 burn {burned_ickb} and later normalized claim value {later_ickb_value} should differ by at most one shannon"
        );
        burned_ickb
    };

    let older_deposit_burn = run_case(1_000 * CKB, 1554, GENESIS_AR as u64);
    let newer_deposit_burn = run_case(1_000 * CKB, 1555, 11_000_000_000_000_000u64);
    let oversized_deposit_burn = run_case(150_000 * CKB, 1556, 11_000_000_000_000_000u64);

    assert!(
        older_deposit_burn > newer_deposit_burn,
        "the older deposit should require more nominal iCKB because it is worth more iCKB"
    );
    assert!(
        oversized_deposit_burn > newer_deposit_burn,
        "the oversized deposit should still cost more iCKB than the smaller newer deposit after the haircut"
    );
}

/// Phase 1 burns exactly the deposit's iCKB value, then phase 2 claims the DAO maximum.
/// Returns (burned iCKB, claimed capacity, deposit occupied capacity).
fn burn_and_claim(
    context: &mut Context,
    (env, user_lock): (&Env, &Script),
    (withdraw_header, withdraw_ar): (&HeaderView, u64),
    (amount, deposit_number, deposit_ar): (u64, u64, u64),
) -> (u128, u64, u64) {
    let deposit_header = gen_header(deposit_number, deposit_ar, 35, 1000, 1000);
    let deposit = env.deposit(&env.ickb, amount);
    let deposit_occupied_capacity = deposit.capacity - amount;
    let burned_ickb = soft_capped_ickb(amount, deposit_ar);
    // The user lock has empty args, like the deposit's ickb_logic lock, so the request occupies
    // exactly what the deposit did and the claim counts exactly the deposit's amount.
    let request = request(&deposit, user_lock, deposit_number);

    let phase1_tx = TxSpec {
        inputs: vec![In::new(deposit).in_block(&deposit_header), In::new((Cell { lock: user_lock.clone(), ..env.udt(burned_ickb) }).sized())],
        outputs: vec![request.clone().into()],
        header_deps: vec![deposit_header.clone()],
    }
    .build(context);
    context
        .verify(&phase1_tx, MAX_CYCLES)
        .expect("exact phase1 burn should verify for the deposit header");

    let claim_capacity = dao_maximum_withdraw_capacity(&request.output(), request.data.len(), deposit_ar, withdraw_ar);
    // The claim spends the request at the out point phase 1 created.
    let claim = TxSpec {
        inputs: vec![In {
            out_point: OutPoint::new(phase1_tx.hash(), 0),
            header: Some(withdraw_header.clone()),
            since: 0x2003e800000002f4,
            witness: header_dep_index_witness(1),
            cell: request,
        }],
        outputs: vec![Cell { lock: user_lock.clone(), ..env.plain(claim_capacity) }.into()],
        header_deps: vec![withdraw_header.clone(), deposit_header],
    };
    claim.verify(context).expect("phase2 claim should verify for the withdrawal request");

    (burned_ickb, claim_capacity, deposit_occupied_capacity)
}

// The iCKB value counts only unoccupied capacity, while the DAO claim also returns the occupied 82 CKB:
// per iCKB burned, a small deposit returns more CKB than a standard one. This is the whitepaper's
// intended penalty on small deposits, collected by whoever withdraws them.
#[test]
fn smaller_deposits_return_more_ckb_per_ickb_burned_by_their_occupied_capacity() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user_lock = always_success_lock(&mut context);
    let withdraw_ar = 12_000_000_000_000_000u64;
    let withdraw_header = gen_header(2_000_610, withdraw_ar, 575, 2_000_000, 1100);
    let deposit_ar = 11_000_000_000_000_000u64;

    let mut ckb_per_ickb = |amount: u64, deposit_number: u64| {
        // The verified claim is the unoccupied capacity grown by the AR ratio, plus the occupied capacity.
        let (burned, claimed, _) = burn_and_claim(
            &mut context,
            (&env, &user_lock),
            (&withdraw_header, withdraw_ar),
            (amount, deposit_number, deposit_ar),
        );
        (claimed, burned)
    };
    let (small_claim, small_burn) = ckb_per_ickb(1_000 * CKB, 1554);
    let (standard_claim, standard_burn) = ckb_per_ickb(100_000 * CKB, 1555);

    // Compare claim / burn without division: small_claim * standard_burn > standard_claim * small_burn.
    assert!(u128::from(small_claim) * standard_burn > u128::from(standard_claim) * small_burn);
}
