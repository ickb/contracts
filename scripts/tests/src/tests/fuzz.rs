use super::*;
use ckb_dao_utils::extract_dao_data;
use ckb_testtool::ckb_types::core::HeaderView;
use std::collections::BTreeMap;

mod exhaustive;
mod generate;
mod liveness;
mod mainnet;
mod mutate;
mod oracle;
mod reach;

use generate::*;
use mutate::*;
use oracle::*;
pub(super) use oracle::check_invariants;

// Structured fuzzing of the deployed binaries: each case composes valid iCKB, owned-owner and
// limit-order transaction segments, applies a few field, count and ordering mutations, verifies
// through `VerifyRealistic`, and checks the protocol invariants on every accepted transaction.
// The binaries are frozen, so this is evidence about them, not regression protection; the default
// budget stays small for CI, and `ICKB_FUZZ_CASES` / `ICKB_FUZZ_SEED` drive long local runs.
// `ICKB_FUZZ_CASE_SEED` replays one case from the seed a failure prints.

const DEFAULT_SEED: u64 = 0x1c6b_f022_5eed_0001;
// Sized so the debug-profile suite stays within a few seconds; long runs use --release.
const DEFAULT_CASES: u64 = 100;
// Above MAX_CYCLES so composed transactions are not rejected for size alone.
const FUZZ_MAX_CYCLES: u64 = 100_000_000;

// Each script's `Error` enum in declaration order; code n is entry n - 1.
const SCRIPT_ERRORS: [(&str, &[&str]); 3] = [
    (
        "ickb_logic",
        &[
            "IndexOutOfBound", "ItemMissing", "LengthNotEnough", "Encoding", "NotEmptyArgs", "ScriptMisuse",
            "DepositTooSmall", "DepositTooBig", "EmptyReceipt", "ReceiptMismatch", "AmountMismatch",
            "AmountUnreasonablyBig",
        ],
    ),
    (
        "owned_owner",
        &[
            "IndexOutOfBound", "ItemMissing", "LengthNotEnough", "Encoding", "NotEmptyArgs", "NotWithdrawalRequest",
            "ScriptMisuse", "Mismatch",
        ],
    ),
    (
        "limit_order",
        &[
            "IndexOutOfBound", "ItemMissing", "LengthNotEnough", "Encoding", "NotEmptyArgs", "DuplicatedMaster",
            "InvalidAction", "NonZeroPadding", "InvalidRatio", "InvalidCkbMinMatchLog", "ConcaveRatio",
            "BothRatioNull", "MissingUdtType", "SameMaster", "ScriptMisuse", "DifferentInfo", "InvalidMatch",
            "DecreasingValue", "AttemptToChangeFulfilled", "InsufficientMatch", "InvalidConfiguration",
        ],
    ),
];

pub(super) struct Rng(u64);

impl Rng {
    // splitmix64: tiny, deterministic, and good enough to pick shapes and mutations.
    pub(super) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    pub(super) fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }

    pub(super) fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }

    pub(super) fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }

    pub(super) fn index(&mut self, len: usize) -> usize {
        self.below(len as u64) as usize
    }

    pub(super) fn hash(&mut self) -> [u8; 32] {
        let mut h = [0u8; 32];
        for chunk in h.chunks_mut(8) {
            chunk.copy_from_slice(&self.next().to_le_bytes());
        }
        h
    }

    pub(super) fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.index(i + 1);
            v.swap(i, j);
        }
    }
}

#[derive(Default)]
struct Stats {
    // Per segment kind: (unmutated accepted, mutated accepted, mutated rejected).
    kinds: BTreeMap<&'static str, [u64; 3]>,
    unrealistic: u64,
    errors: BTreeMap<String, u64>,
}

enum Outcome {
    Unrealistic,
    Rejected(String),
    Accepted,
}

fn run_case(case_seed: u64, stats: Option<&mut Stats>) -> Result<(), String> {
    let mut rng = Rng(case_seed);
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let (mut spec, kinds) = generate_case(&mut rng, &env);
    let mutations = if rng.chance(1, 5) { 0 } else { rng.range(1, 3) };
    let mut applied = Vec::new();
    for _ in 0..mutations {
        applied.push(mutate(&mut rng, &env, &mut spec));
    }
    let tx = spec.balance().then(|| spec.build(&mut context));
    let outcome = match tx.filter(|tx| context.node_rejection(tx).is_none()) {
        None => Outcome::Unrealistic,
        Some(tx) => match context.verify(&tx, FUZZ_MAX_CYCLES) {
            Err(err) => Outcome::Rejected(error_class(&env, &spec, &err.to_string())),
            Ok(_) => {
                let violations = check_invariants(&context, &tx);
                if !violations.is_empty() {
                    return Err(format!(
                        "case seed {case_seed:#x}: accepted transaction breaks invariants\n  segments: {kinds:?}\n  mutations: {applied:?}\n  violations:\n    {}\n  transaction:\n{}",
                        violations.join("\n    "),
                        describe(&env, &spec)
                    ));
                }
                Outcome::Accepted
            }
        },
    };
    if mutations == 0 {
        let err = match &outcome {
            Outcome::Rejected(e) => Some(e.clone()),
            Outcome::Unrealistic => Some("unrealistic".to_string()),
            Outcome::Accepted => None,
        };
        if let Some(err) = err {
            return Err(format!(
                "case seed {case_seed:#x}: unmutated segments {kinds:?} should verify but got {err}\n{}",
                describe(&env, &spec)
            ));
        }
    }
    if let Some(stats) = stats {
        match &outcome {
            Outcome::Unrealistic => stats.unrealistic += 1,
            Outcome::Rejected(err) => *stats.errors.entry(err.clone()).or_default() += 1,
            Outcome::Accepted => (),
        }
        for kind in kinds {
            let counts = stats.kinds.entry(kind).or_default();
            match (&outcome, mutations) {
                (Outcome::Accepted, 0) => counts[0] += 1,
                (Outcome::Accepted, _) => counts[1] += 1,
                (Outcome::Rejected(_), _) => counts[2] += 1,
                (Outcome::Unrealistic, _) => (),
            }
        }
    }
    Ok(())
}

/// The failing script and its error code, so the summary shows which checks fire.
fn error_class(env: &Env, spec: &TxSpec, err: &str) -> String {
    let source = err
        .split("TransactionScriptError { source: ")
        .nth(1)
        .and_then(|s| s.split([' ', ',']).next())
        .unwrap_or("?");
    let code = err.split("error code ").nth(1).and_then(|s| s.split(' ').next()).unwrap_or("?");
    let index = |s: &str| s.split(['[', ']']).nth(1).and_then(|i| i.parse::<usize>().ok());
    let script = match (source.starts_with("Inputs"), source.ends_with("Lock"), index(source)) {
        (true, true, Some(i)) => spec.inputs.get(i).map(|c| c.cell.lock.clone()),
        (true, false, Some(i)) => spec.inputs.get(i).and_then(|c| c.cell.type_.clone()),
        (false, _, Some(i)) => spec.outputs.get(i).and_then(|c| c.cell.type_.clone()),
        _ => None,
    };
    let role = if source.ends_with("Lock") { "lock" } else { "type" };
    match script {
        Some(s) => format!("{} as {role}, code {code}", script_name(env, &s)),
        None => format!("{source}, {}", err.chars().take(80).collect::<String>()),
    }
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().map(|v| {
        let v = v.trim();
        match v.strip_prefix("0x") {
            Some(hex) => u64::from_str_radix(hex, 16),
            None => v.parse(),
        }
        .unwrap_or_else(|_| panic!("{name} must be a decimal or 0x-hex u64"))
    })
}

#[test]
fn fuzz_accepted_transactions_keep_protocol_invariants() {
    if let Some(case_seed) = env_u64("ICKB_FUZZ_CASE_SEED") {
        run_case(case_seed, None).unwrap_or_else(|e| panic!("{e}"));
        return;
    }
    let seed = env_u64("ICKB_FUZZ_SEED").unwrap_or(DEFAULT_SEED);
    let cases = env_u64("ICKB_FUZZ_CASES").unwrap_or(DEFAULT_CASES);
    let mut master = Rng(seed);
    let mut stats = Stats::default();
    for _ in 0..cases {
        let case_seed = master.next();
        run_case(case_seed, Some(&mut stats)).unwrap_or_else(|e| panic!("{e}"));
    }
    println!("fuzz seed {seed:#x}, {cases} cases, {} unrealistic", stats.unrealistic);
    for (kind, [base, accepted, rejected]) in &stats.kinds {
        println!("  {kind}: {base} unmutated accepted, {accepted} mutated accepted, {rejected} mutated rejected");
    }
    for (err, n) in &stats.errors {
        println!("  rejected by {err}: {n}");
    }
    // Coverage proxy: an error code no case triggered is a validation branch this run never tested.
    for (script, codes) in SCRIPT_ERRORS {
        let unreached: Vec<&str> = codes
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                // The same code with non-empty args counts too: it is the one that hits NotEmptyArgs.
                let reached = |name: &str, role: &str| stats.errors.contains_key(&format!("{name} as {role}, code {}", i + 1));
                let sibling = format!("{script}(args)");
                !["lock", "type"].iter().any(|role| reached(script, role) || reached(&sibling, role))
            })
            .map(|(_, name)| *name)
            .collect();
        println!("  {script} error codes never reached: {unreached:?}");
    }
    // Health check: a fixture change that leaves the fuzzer running but checking nothing fails here.
    // Every segment kind must verify unmutated and be rejected once mutated, and some mutated
    // transactions must reach the oracle.
    for kind in SEGMENT_KINDS {
        let [base, _, rejected] = stats.kinds.get(kind).copied().unwrap_or_default();
        assert!(base > 0 && rejected > 0, "segment {kind}: {base} unmutated accepted, {rejected} mutated rejected");
    }
    assert!(stats.kinds.values().any(|[_, accepted, _]| *accepted > 0), "no mutated transaction was accepted");
}

/// One line per cell, literal enough to rebuild the case by hand.
fn describe(env: &Env, spec: &TxSpec) -> String {
    let name = |s: &Script| script_name(env, s);
    let line = |c: &Cell| {
        format!(
            "capacity {} lock {} type {} data {}",
            c.capacity,
            name(&c.lock),
            c.type_.as_ref().map_or("none".to_string(), |t| name(t)),
            hex::encode(&c.data)
        )
    };
    let mut s = String::new();
    for (i, input) in spec.inputs.iter().enumerate() {
        let index: u32 = input.out_point.index().unpack();
        s += &format!(
            "    input {i}: out_point {}:{index} since {:#x} header {} witness {} | {}\n",
            hex::encode(&input.out_point.tx_hash().as_slice()[..4]),
            input.since,
            input.header.as_ref().map_or("none".to_string(), |h| format!(
                "#{} ar {}",
                h.number(),
                extract_dao_data(h.dao()).0
            )),
            hex::encode(&input.witness),
            line(&input.cell)
        );
    }
    for (i, output) in spec.outputs.iter().enumerate() {
        s += &format!("    output {i}: {}\n", line(&output.cell));
    }
    for h in &spec.header_deps {
        s += &format!("    header dep #{} ar {}\n", h.number(), extract_dao_data(h.dao()).0);
    }
    s
}

fn script_name(env: &Env, s: &Script) -> String {
    let h = s.calc_script_hash();
    if h == env.user.calc_script_hash() {
        "user".to_string()
    } else if h == env.funding.calc_script_hash() {
        "funding".to_string()
    } else if h == env.ickb.calc_script_hash() {
        "ickb_logic".to_string()
    } else if h == env.dao.calc_script_hash() {
        "dao".to_string()
    } else if h == env.xudt.calc_script_hash() {
        "ickb_xudt".to_string()
    } else if h == env.owned_owner.calc_script_hash() {
        "owned_owner".to_string()
    } else if h == env.limit_order.calc_script_hash() {
        "limit_order".to_string()
    } else if h == env.helper.calc_script_hash() {
        "helper_type".to_string()
    } else {
        // The siblings share code with ickb_logic, owned_owner, limit_order and xUDT, in Env's order.
        const SIBLINGS: [&str; 8] = [
            "ickb_logic(args)",
            "owned_owner(args)",
            "limit_order(args)",
            "xudt(args)",
            "ickb_logic(data2)",
            "owned_owner(data2)",
            "limit_order(data2)",
            "xudt(data2)",
        ];
        match env.siblings.iter().position(|sibling| sibling.calc_script_hash() == h) {
            Some(i) => SIBLINGS[i].to_string(),
            None => format!("other:{}", hex::encode(&h.as_slice()[..4])),
        }
    }
}
