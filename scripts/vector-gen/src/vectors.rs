//! The golden protocol vectors: the cases and the values the contracts' own Rust
//! logic gives them. Shared by the JSON writer (`main.rs`) and the contract tests,
//! which replay its rows against the release binaries (`scripts/tests`).
//!
//! Provenance: `c256.rs` and `ickb_logic/constants.rs` are compiled directly from
//! the contract sources via `#[path]`. `deposit_to_ickb` and `validate` are
//! private, syscall-bound functions, so they are included as byte-exact verbatim
//! copies (src/copied/*); `drift_checks` asserts each copy is still a substring of
//! its source file.

// Contract source compiled directly (not copied).
#[allow(dead_code)]
#[path = "../../contracts/utils/src/c256.rs"]
mod c256;

// Contract source compiled directly (not copied).
#[allow(dead_code)]
#[path = "../../contracts/ickb_logic/src/constants.rs"]
mod ickb_constants;

/// iCKB deposit conversion, around the verbatim copy of `deposit_to_ickb`
/// (scripts/contracts/ickb_logic/src/entry.rs lines 71-84).
pub mod ickb {
    use super::ickb_constants::ICKB_SOFT_CAP_PER_DEPOSIT;

    // Verbatim value from scripts/contracts/utils/src/constants.rs
    // (that file cannot be #[path]-included: it depends on ckb-std).
    // Drift-checked by substring in drift_checks().
    pub const GENESIS_ACCUMULATED_RATE: u128 = 10_000_000_000_000_000; // 10^16 Genesis block accumulated rate

    #[derive(Debug)]
    pub enum Error {}

    // Off-chain stand-in: on-chain, `extract_accumulated_rate(index, source)`
    // reads AR_m from the deposit's header dep. Here the Source carries it.
    #[derive(Clone, Copy)]
    pub struct Source {
        pub ar: u64,
    }

    fn extract_accumulated_rate(_index: usize, source: Source) -> Result<u64, Error> {
        Ok(source.ar)
    }

    include!("copied/deposit_to_ickb.rs");

    pub fn compute(ar: u64, amount: u64) -> u128 {
        deposit_to_ickb(0, Source { ar }, amount).unwrap()
    }
}

/// Limit order match validation, around the verbatim copies of `validate` and
/// the order data types (scripts/contracts/limit_order/src/entry.rs lines
/// 86-133 and 141-161).
pub mod limit_order {
    use super::c256::C256;

    // Same variant names as scripts/contracts/limit_order/src/error.rs
    // (name presence drift-checked in drift_checks()); only the variants reachable
    // from `validate` are needed here.
    #[derive(Debug, PartialEq)]
    pub enum Error {
        DifferentInfo,
        InvalidMatch,
        DecreasingValue,
        AttemptToChangeFulfilled,
        InsufficientMatch,
    }

    include!("copied/limit_order_types.rs");
    include!("copied/limit_order_validate.rs");

    /// (ckb capacity, udt amount, unoccupied ckb) of one order cell.
    pub type Side = (u64, u128, u64);

    pub fn run(
        c2u: Option<(u64, u64)>,
        u2c: Option<(u64, u64)>,
        min_log: u8,
        i: Side,
        o: Side,
    ) -> Result<(), Error> {
        assert!(min_log <= 64, "ckb_min_match_log out of the parser's 0..=64 range");
        let ratio = |r: Option<(u64, u64)>| {
            r.map(|(ckb_mul, udt_mul)| Ratio {
                ckb_mul: C256::from(ckb_mul),
                udt_mul: C256::from(udt_mul),
            })
        };
        let info = Info {
            udt_hash: [0u8; 32],
            ckb_to_udt: ratio(c2u),
            udt_to_ckb: ratio(u2c),
            // Same computation as the order parser: `n @ 0..=64 => C256::from(1u128 << n)`
            ckb_min_match: C256::from(1u128 << min_log),
        };
        let data = |(ckb, udt, unoccupied): Side| Data {
            ckb: C256::from(ckb),
            udt: C256::from(udt),
            ckb_unoccupied: C256::from(unoccupied),
            info,
        };
        validate(data(i), data(o))
    }
}

use ickb_constants::{
    CKB_MAXIMUM_UNOCCUPIED_CAPACITY_PER_DEPOSIT as MAX_DEPOSIT,
    CKB_MINIMUM_UNOCCUPIED_CAPACITY_PER_DEPOSIT as MIN_DEPOSIT, ICKB_SOFT_CAP_PER_DEPOSIT,
};

const STD_DEPOSIT: u64 = 100_000 * 100_000_000; // 100_000 CKB, the standard deposit size
const CKB_250K: u64 = 250_000 * 100_000_000; // mid discount region

const AR_GENESIS: u64 = 10_000_000_000_000_000;
const AR_105: u64 = 10_500_000_000_000_000; // realistic 1.05e16
const AR_118: u64 = 11_800_000_000_000_000; // realistic 1.18e16

/// Largest unoccupied capacity whose conversion at `ar` stays at or under the
/// soft cap (so it takes no discount); +1 shannon crosses the cap.
fn softcap_boundary(ar: u64) -> u64 {
    let ar0 = ickb::GENESIS_ACCUMULATED_RATE;
    let x = ((ICKB_SOFT_CAP_PER_DEPOSIT + 1) * u128::from(ar) - 1) / ar0;
    let x = u64::try_from(x).unwrap();
    assert!(u128::from(x) * ar0 / u128::from(ar) <= ICKB_SOFT_CAP_PER_DEPOSIT);
    assert!(u128::from(x + 1) * ar0 / u128::from(ar) > ICKB_SOFT_CAP_PER_DEPOSIT);
    x
}

pub fn drift_checks() {
    let ickb_entry = include_str!("../../contracts/ickb_logic/src/entry.rs");
    assert!(
        ickb_entry.contains(include_str!("copied/deposit_to_ickb.rs").trim_end()),
        "copied deposit_to_ickb drifted from ickb_logic/src/entry.rs"
    );
    let lo_entry = include_str!("../../contracts/limit_order/src/entry.rs");
    assert!(
        lo_entry.contains(include_str!("copied/limit_order_validate.rs").trim_end()),
        "copied validate drifted from limit_order/src/entry.rs"
    );
    assert!(
        lo_entry.contains(include_str!("copied/limit_order_types.rs").trim_end()),
        "copied order types drifted from limit_order/src/entry.rs"
    );
    assert!(
        include_str!("../../contracts/utils/src/constants.rs")
            .contains("pub const GENESIS_ACCUMULATED_RATE: u128 = 10_000_000_000_000_000;"),
        "GENESIS_ACCUMULATED_RATE drifted from utils/src/constants.rs"
    );
    let lo_errors = include_str!("../../contracts/limit_order/src/error.rs");
    for variant in [
        "DifferentInfo",
        "InvalidMatch",
        "DecreasingValue",
        "AttemptToChangeFulfilled",
        "InsufficientMatch",
    ] {
        assert!(lo_errors.contains(variant), "Error variant {variant} missing from limit_order/src/error.rs");
    }
}


/// One `depositToIckb` vector: a receipt of `amount` unoccupied shannons at `ar`.
pub struct DepositRow {
    pub ar: u64,
    pub amount: u64,
    pub expected_ickb: u128,
    pub note: String,
}

/// One `limitOrderMatch` vector and the verdict of the copied `validate`.
pub struct LimitRow {
    pub ckb_to_udt: Option<(u64, u64)>,
    pub udt_to_ckb: Option<(u64, u64)>,
    pub ckb_min_match_log: u8,
    pub input: limit_order::Side,
    pub output: limit_order::Side,
    pub verdict: Result<(), limit_order::Error>,
    pub note: &'static str,
}

pub fn deposit_rows() -> Vec<DepositRow> {
    rows().0
}

pub fn limit_rows() -> Vec<LimitRow> {
    rows().1
}

fn rows() -> (Vec<DepositRow>, Vec<LimitRow>) {
    // ---------------- depositToIckb ----------------
    let mut deposit_rows: Vec<(u64, u64, String)> = Vec::new();
    for (ar, ar_note) in [
        (AR_GENESIS, "genesis AR 1e16 (identity)"),
        (AR_105, "AR 1.05e16"),
        (AR_118, "AR 1.18e16"),
    ] {
        let boundary = softcap_boundary(ar);
        for (amount, note) in [
            (MIN_DEPOSIT, "minimum deposit bound, 1000 CKB unoccupied"),
            (5 * MIN_DEPOSIT, "mid-range 5000 CKB"),
            (STD_DEPOSIT, "standard 100k CKB deposit"),
            (boundary, "soft-cap boundary: largest amount with no discount"),
            (boundary + 1, "one shannon past the soft-cap boundary"),
            (CKB_250K, "mid discount region, 250k CKB"),
            (MAX_DEPOSIT, "maximum deposit bound, 1M CKB unoccupied"),
        ] {
            deposit_rows.push((ar, amount, format!("{note} at {ar_note}")));
        }
    }
    deposit_rows.push((
        AR_GENESIS + 1,
        STD_DEPOSIT,
        "AR one above genesis: floor drops one shannon".into(),
    ));
    deposit_rows.push((2 * AR_GENESIS, STD_DEPOSIT, "doubled AR halves iCKB for standard deposit".into()));
    deposit_rows.push((2 * AR_GENESIS, MAX_DEPOSIT, "doubled AR, max deposit, discount region".into()));

    // ---------------- limitOrderMatch ----------------
    // (ckb_to_udt, udt_to_ckb, ckb_min_match_log, input, output, note)
    type LRow = (
        Option<(u64, u64)>,
        Option<(u64, u64)>,
        u8,
        limit_order::Side,
        limit_order::Side,
        &'static str,
    );
    #[rustfmt::skip]
    let limit_rows: Vec<LRow> = vec![
        // ckb2udt basics, ratio 2:1, min-match log 10 (1024), occupied 100
        (Some((2, 1)), None, 10, (10_100, 0, 10_000), (8_076, 4_048, 7_976), "ckb2udt 2:1 equal-value partial match"),
        (Some((2, 1)), None, 10, (10_100, 0, 10_000), (8_076, 4_047, 7_976), "ckb2udt: one udt shannon short of equal value"),
        (Some((2, 1)), None, 10, (10_100, 0, 10_000), (8_076, 4_049, 7_976), "ckb2udt: matcher overpays one udt shannon"),
        (Some((2, 1)), None, 10, (10_100, 0, 10_000), (9_076, 2_048, 8_976), "ckb2udt: partial match exactly at ckb_min_match 1024"),
        (Some((2, 1)), None, 10, (10_100, 0, 10_000), (9_077, 2_046, 8_977), "ckb2udt: one shannon below ckb_min_match"),
        (Some((2, 1)), None, 10, (200, 0, 100), (100, 200, 0), "ckb2udt: below-min final fill allowed when output is fulfilled"),
        (Some((2, 1)), None, 10, (10_100, 4_048, 0), (10_000, 4_248, 0), "ckb2udt: value-preserving change of fulfilled order rejected"),
        (Some((2, 1)), None, 10, (201, 0, 1), (200, 2, 0), "ckb2udt: single-shannon completion of nearly fulfilled order"),
        (Some((2, 1)), None, 10, (10_100, 0, 0), (10_000, 100, 0), "check order: DecreasingValue precedes AttemptToChangeFulfilled"),
        // udt2ckb basics, ratio 1:1, min-match log 10, occupied 100
        (None, Some((1, 1)), 10, (10_100, 50_000, 10_000), (30_100, 30_000, 30_000), "udt2ckb 1:1 equal-value partial match"),
        (None, Some((1, 1)), 10, (10_100, 50_000, 10_000), (30_099, 30_000, 29_999), "udt2ckb: one ckb shannon short of equal value"),
        (None, Some((1, 1)), 10, (10_100, 50_000, 10_000), (11_124, 48_976, 11_024), "udt2ckb: udt moved exactly the min-match equivalent 1024"),
        (None, Some((1, 1)), 10, (10_100, 50_000, 10_000), (11_123, 48_977, 11_023), "udt2ckb: one udt below the min-match equivalent"),
        (None, Some((1, 1)), 10, (200, 10, 100), (210, 0, 110), "udt2ckb: below-min final fill allowed when udt is exhausted"),
        (None, Some((1, 1)), 10, (10_000, 0, 9_900), (10_100, 0, 10_000), "fulfilled udt2ckb: udt=0 matches no direction arm, so InvalidMatch, never AttemptToChangeFulfilled"),
        // C1 counterexample: ratio 2:1, min-match log 3 (8)
        (Some((2, 1)), None, 3, (1_100, 0, 1_000), (1_098, 4, 998), "C1: moving 2 ckb is below ckb_min_match 8"),
        (Some((2, 1)), None, 3, (1_100, 0, 1_000), (1_092, 16, 992), "C1: correct minimum, 8 ckb for 16 udt"),
        // asymmetric small ratio 2:3, min-match log 10 -> udt threshold ceil(2048/3) = 683
        (None, Some((2, 3)), 10, (10_100, 10_000, 10_000), (11_125, 9_317, 11_025), "asymmetric 2:3: 683 udt meets the min-match equivalent"),
        (None, Some((2, 3)), 10, (10_100, 10_000, 10_000), (11_123, 9_318, 11_023), "asymmetric 2:3: 682 udt is one below the min-match equivalent"),
        // asymmetric large multipliers (1e9 scale), min-match log 0
        (Some((1_000_000_007, 730_000_001)), None, 0, (10_000_000_000, 0, 9_999_999_900), (9_269_999_999, 1_000_000_007, 9_269_999_899), "large multipliers: exact equal value ckb2udt"),
        (Some((1_000_000_007, 730_000_001)), None, 0, (10_000_000_000, 0, 9_999_999_900), (9_269_999_999, 1_000_000_006, 9_269_999_899), "large multipliers: one udt shannon short"),
        (None, Some((999_983, 1_299_709)), 0, (10_000_000_000, 2_000_000, 9_999_999_900), (10_001_299_709, 1_000_017, 10_001_299_609), "large multipliers: exact equal value udt2ckb"),
        // 256-bit regime: udt near u128 max, value products exceed u128
        (None, Some((1_000, 1_000_000_000_000_000)), 0, (10_000, 330_000_000_000_000_000_000_000_000_000_000_000_000, 9_900), (1_000_000_000_010_000, 329_999_999_999_999_999_999_999_999_999_999_999_000, 1_000_000_000_009_900), "256-bit regime: value products exceed u128, C256 stays exact"),
        (None, Some((1_000, 1_000_000_000_000_000)), 0, (10_000, 330_000_000_000_000_000_000_000_000_000_000_000_000, 9_900), (1_000_000_000_009_999, 329_999_999_999_999_999_999_999_999_999_999_999_000, 1_000_000_000_009_899), "256-bit regime: one ckb shannon short"),
        // dual-ratio orders: ckb_to_udt 2:1 and udt_to_ckb 1:1, min-match log 0
        (Some((2, 1)), Some((1, 1)), 0, (1_100, 0, 1_000), (1_000, 150, 900), "dual ratio: ckb-decrease priced by ckb_to_udt 2:1, 150 udt insufficient"),
        (Some((2, 1)), Some((1, 1)), 0, (1_100, 0, 1_000), (1_000, 200, 900), "dual ratio: 200 udt satisfies ckb_to_udt 2:1"),
        (Some((2, 1)), Some((1, 1)), 0, (1_100, 300, 1_000), (1_160, 200, 1_060), "dual ratio: udt-decrease priced by udt_to_ckb 1:1, 60 ckb insufficient though 2:1 would pass"),
        (Some((2, 1)), Some((1, 1)), 0, (1_100, 300, 1_000), (1_200, 200, 1_100), "dual ratio: udt_to_ckb side exact equal value"),
        // invalid configurations
        (None, Some((1, 1)), 10, (1_100, 0, 1_000), (1_000, 100, 900), "sell-ckb move against a udt2ckb-only order"),
        (Some((1, 1)), None, 10, (1_000, 100, 900), (1_100, 0, 1_000), "sell-udt move against a ckb2udt-only order"),
        (Some((1, 1)), Some((1, 1)), 10, (1_100, 100, 1_000), (1_100, 100, 1_000), "no-op match rejected"),
        (Some((1, 1)), Some((1, 1)), 10, (1_100, 100, 1_000), (1_000, 50, 900), "both sides decrease"),
        (Some((1, 1)), Some((1, 1)), 10, (1_000, 50, 900), (1_100, 100, 1_000), "both sides increase"),
        // min-match log extremes
        (Some((1, 1)), None, 64, (2_000, 0, 1_900), (1_000, 1_000, 900), "min-match log 64: any partial ckb2udt is insufficient"),
        (Some((1, 1)), None, 64, (2_000, 0, 1_900), (100, 1_900, 0), "min-match log 64: completion bypasses the check"),
        (Some((1, 1)), None, 0, (1_100, 0, 1_000), (1_099, 1, 999), "min-match log 0: single-shannon partial match allowed"),
    ];


    let deposits = deposit_rows
        .into_iter()
        .map(|(ar, amount, note)| DepositRow { ar, amount, expected_ickb: ickb::compute(ar, amount), note })
        .collect();
    let limits = limit_rows
        .into_iter()
        .map(|(ckb_to_udt, udt_to_ckb, ckb_min_match_log, input, output, note)| LimitRow {
            ckb_to_udt,
            udt_to_ckb,
            ckb_min_match_log,
            input,
            output,
            verdict: limit_order::run(ckb_to_udt, udt_to_ckb, ckb_min_match_log, input, output),
            note,
        })
        .collect();
    (deposits, limits)
}
