//! Reports which instructions of the deployed iCKB binaries a test run executed.
//!
//! Usage, from scripts/tests:
//!   cargo run --locked --bin coverage -- <release_dir> <symbols_dir> [--functions] [--runs] [--annotate <binary> <text>] <trace files...>
//!
//! <release_dir> holds the deployed binaries and <symbols_dir> their relinked copies with symbols
//! (<name>.sym), both as run.sh leaves them. A trace file has one "<program fnv64> <pc> <length>"
//! line per instruction the instrumented VM decoded, see coverage/README.md.
//!
//! For each binary it prints the instructions executed, then the never-executed ones in three groups:
//! functions never entered, runs that end in a panic or an allocation failure, and the rest.
//! --functions lists every function not fully executed, --runs prints the rest group as disassembly,
//! --annotate prints the functions whose name contains <text> with "--" on each never-executed instruction.
use std::collections::{HashMap, HashSet};
use std::process::Command;

const BINARIES: [&str; 3] = ["ickb_logic", "limit_order", "owned_owner"];
/// Calls to these never return: a run that reaches one of them stops the script.
const STOPS: [&str; 7] = ["panic", "_fail", "handle_alloc_error", "capacity_overflow", "__rust_alloc_error_handler", "rdl_oom", "rt_error"];
const GROUPS: [&str; 3] = ["in functions never entered", "on paths that end in a panic or an allocation failure", "in the rest"];

type Function = (String, Vec<(u64, String)>);

fn fnv64(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &x| (h ^ u64::from(x)).wrapping_mul(0x100_0000_01b3))
}

fn mnemonic(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or("")
}

fn is_call(text: &str) -> bool {
    matches!(mnemonic(text), "jalr" | "jal" | "call")
}

fn is_stop(text: &str) -> bool {
    is_call(text) && STOPS.iter().any(|s| text.contains(s))
}

/// Shortens each long `<symbol>` operand to its first 60 characters.
fn short(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..=open]);
        rest = &rest[open + 1..];
        match rest.find('>') {
            Some(close) if rest[..close].chars().count() >= 60 => {
                out.extend(rest[..close].chars().take(60));
                out.push_str("..>");
                rest = &rest[close + 1..];
            }
            _ => {}
        }
    }
    out + rest
}

fn name(fname: &str) -> String {
    fname.chars().take(140).collect()
}

/// The functions of the symbolized binary with their instructions, local labels folded into their function.
fn functions(path: &str) -> Vec<Function> {
    let out = Command::new("llvm-objdump").args(["-d", "--no-show-raw-insn", "-C", path]).output().expect("run llvm-objdump");
    assert!(out.status.success(), "llvm-objdump failed on {path}");
    let mut funcs: Vec<Function> = Vec::new();
    for line in String::from_utf8(out.stdout).expect("utf-8").lines() {
        if let Some(label) = line.strip_suffix(">:").and_then(|l| l.split_once(" <")).filter(|(a, _)| a.len() == 16) {
            if !label.1.starts_with(".L") {
                funcs.push((label.1.to_string(), Vec::new()));
            }
        } else if let Some((address, text)) = line.split_once(':').filter(|(a, _)| a.starts_with(char::is_whitespace)) {
            let text = text.trim();
            // `unimp` pads the end of a function after a call that never returns.
            if let (Some(f), Ok(address)) = (funcs.last_mut(), u64::from_str_radix(address.trim(), 16)) {
                if !text.starts_with("unimp") {
                    f.1.push((address, text.to_string()));
                }
            }
        }
    }
    funcs
}

/// Whether straight-line execution from instruction `i` of function `fi` reaches a call that never returns.
fn stops(funcs: &[Function], at: &HashMap<u64, (usize, usize)>, fi: usize, mut i: usize, depth: u32) -> bool {
    let ins = &funcs[fi].1;
    while i < ins.len() {
        let text = &ins[i].1;
        if is_stop(text) {
            return true;
        }
        let m = mnemonic(text);
        if m == "j" {
            let target = text.split_whitespace().nth(1).and_then(|t| t.strip_prefix("0x")).and_then(|t| u64::from_str_radix(t, 16).ok());
            return match target.and_then(|t| at.get(&t)) {
                Some(&(f, k)) => depth < 6 && stops(funcs, at, f, k, depth + 1),
                None => false,
            };
        }
        if matches!(m, "ret" | "jr" | "jalr" | "jal" | "call") || (m.len() > 1 && m.starts_with('b')) {
            return false;
        }
        i += 1;
    }
    false
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    assert!(args.len() >= 2, "usage: coverage <release_dir> <symbols_dir> [options] <trace files...>");
    let (release, symbols) = (args.remove(0), args.remove(0));
    let annotate = args.iter().position(|a| a == "--annotate").map(|i| {
        let a: Vec<String> = args.drain(i..i + 3).collect();
        (a[1].clone(), a[2].clone())
    });
    let show_functions = args.iter().any(|a| a == "--functions");
    let show_runs = args.iter().any(|a| a == "--runs");

    let ids: HashMap<u64, &str> = BINARIES.iter().map(|&b| (fnv64(&std::fs::read(format!("{release}/{b}")).expect("read binary")), b)).collect();
    let mut executed: HashMap<&str, HashSet<u64>> = HashMap::new();
    for path in args.iter().filter(|a| !a.starts_with("--")) {
        for line in std::fs::read_to_string(path).expect("read trace").lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            // A process killed mid-write can leave one short line.
            if let [id, pc, len] = p[..] {
                let (Ok(id), Ok(pc), Ok(len)) = (u64::from_str_radix(id, 16), u64::from_str_radix(pc, 16), len.parse::<u64>()) else { continue };
                if let Some(b) = ids.get(&id) {
                    executed.entry(b).or_default().extend(pc..pc + len);
                }
            }
        }
    }

    for b in BINARIES {
        let funcs = functions(&format!("{symbols}/{b}.sym"));
        let cov = executed.remove(b).unwrap_or_default();
        let hit = |a: &u64| cov.contains(a);
        let at: HashMap<u64, (usize, usize)> =
            funcs.iter().enumerate().flat_map(|(fi, f)| f.1.iter().enumerate().map(move |(i, &(a, _))| (a, (fi, i)))).collect();
        // Every traced address must be an instruction of this binary, or the trace is not its own.
        let stray = cov.iter().filter(|&&a| (0..4).all(|d| a.checked_sub(d).map_or(true, |s| !at.contains_key(&s)))).count();
        assert_eq!(stray, 0, "{b}: {stray} traced addresses are not instructions of this binary");

        let total: usize = funcs.iter().map(|f| f.1.len()).sum();
        let executed_count = funcs.iter().flat_map(|f| &f.1).filter(|(a, _)| hit(a)).count();
        let entered = funcs.iter().filter(|f| f.1.iter().any(|(a, _)| hit(a))).count();
        let mut groups = [0usize; 3];
        let mut rest: Vec<(&str, &[(u64, String)])> = Vec::new();
        for (fi, (fname, ins)) in funcs.iter().enumerate() {
            if !ins.iter().any(|(a, _)| hit(a)) {
                groups[0] += ins.len();
                continue;
            }
            let mut i = 0;
            while i < ins.len() {
                if hit(&ins[i].0) {
                    i += 1;
                    continue;
                }
                // One straight-line piece: up to the next executed instruction or transfer of control.
                let mut j = i;
                while j + 1 < ins.len() && !hit(&ins[j + 1].0) && !matches!(mnemonic(&ins[j].1), "j" | "ret" | "jr") && !is_stop(&ins[j].1) {
                    j += 1;
                }
                if stops(&funcs, &at, fi, i, 0) {
                    groups[1] += j - i + 1;
                } else {
                    groups[2] += j - i + 1;
                    rest.push((fname, &ins[i..=j]));
                }
                i = j + 1;
            }
        }
        let percent = 100.0 * executed_count as f64 / total as f64;
        println!("{b}: {executed_count} of {total} instructions executed ({percent:.1}%), {entered} of {} functions entered", funcs.len());
        for (group, count) in GROUPS.iter().zip(groups) {
            println!("  never executed {group}: {count}");
        }
        if show_functions {
            for (fname, ins) in &funcs {
                let h = ins.iter().filter(|(a, _)| hit(a)).count();
                if h < ins.len() {
                    println!("  {h:5}/{:5} {}", ins.len(), name(fname));
                }
            }
        }
        if show_runs {
            let mut last = None;
            for (fname, run) in rest {
                if last != Some(fname) {
                    println!("\n  ### {}", name(fname));
                    last = Some(fname);
                }
                for (a, text) in run {
                    println!("     {a:x}: {}", short(text));
                }
                println!("     .");
            }
        }
        if let Some((_, text)) = annotate.as_ref().filter(|(binary, _)| binary == b) {
            for (fname, ins) in funcs.iter().filter(|f| f.0.contains(text.as_str())) {
                println!("\n  ### {}", name(fname));
                for (a, t) in ins {
                    println!("{}{a:x}: {}", if hit(a) { "     " } else { "  -- " }, short(t));
                }
            }
        }
    }
}
