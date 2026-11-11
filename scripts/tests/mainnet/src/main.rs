//! Fetches the chain transactions that touch the iCKB scripts as replay fixtures, and checks the live iCKB supply.
//!
//! Usage, from scripts/tests, where the replay test runs too:
//!   cargo run --locked --release --manifest-path mainnet/Cargo.toml -- fetch  ../build/release <out_dir> [rpc_url]
//!   cargo run --locked --release --manifest-path mainnet/Cargo.toml -- supply ../build/release <out_dir> [rpc_url]
//!
//! fetch writes <out_dir>/fixtures.txt (one record per line, molecule hex, read by the Rust replay test),
//! <out_dir>/raw.json with every RPC answer it used, and <out_dir>/meta.json. Every fetched transaction
//! and header is re-serialized here and its hash checked against the RPC hash, so the molecule encoding
//! is proven before it reaches the fixture. The fixtures carry every cell dep too, dep group members
//! included, so the replay can execute each transaction's scripts on the code cells the chain used.
//! Needs curl, sha256sum and date.
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const RPC: &str = "https://mainnet.ckb.dev/rpc";
const XUDT_CODE_HASH: &str = "0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95";
const DAO_CODE_HASH: &str = "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e";
const PAGE: u64 = 1000;
/// Hard cap on indexer entries across all queries.
const TOTAL_BOUND: usize = 1_000_000;
const GENESIS_AR: u128 = 10_000_000_000_000_000;
const CKB: u128 = 100_000_000;

fn hash(b: &[u8]) -> String {
    format!("0x{}", hex(&ckb_hash::blake2b_256(b)))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn text(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("not a string: {v}"))
}

fn h(v: &Value) -> Vec<u8> {
    let s = text(v).strip_prefix("0x").expect("0x prefix");
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex")).collect()
}

fn int(v: &Value) -> u128 {
    u128::from_str_radix(text(v).strip_prefix("0x").expect("0x prefix"), 16).expect("hex number")
}

fn u32(v: &Value) -> [u8; 4] {
    u32::try_from(int(v)).expect("u32").to_le_bytes()
}

fn u64(v: &Value) -> [u8; 8] {
    u64::try_from(int(v)).expect("u64").to_le_bytes()
}

fn le(b: &[u8]) -> u128 {
    b.iter().rev().fold(0, |n, &x| n << 8 | u128::from(x))
}

fn bytes(b: &[u8]) -> Vec<u8> {
    [&(b.len() as u32).to_le_bytes()[..], b].concat()
}

fn fixvec(items: Vec<Vec<u8>>) -> Vec<u8> {
    [(items.len() as u32).to_le_bytes().to_vec(), items.concat()].concat()
}

fn table(fields: Vec<Vec<u8>>) -> Vec<u8> {
    let mut off = 4 + 4 * fields.len() as u32;
    let mut out = Vec::new();
    for f in &fields {
        out.extend(off.to_le_bytes());
        off += f.len() as u32;
    }
    [off.to_le_bytes().to_vec(), out, fields.concat()].concat()
}

fn script(s: &Value) -> Vec<u8> {
    let hash_type = match text(&s["hash_type"]) {
        "data" => 0,
        "type" => 1,
        "data1" => 2,
        "data2" => 4,
        other => panic!("hash type {other}"),
    };
    table(vec![h(&s["code_hash"]), vec![hash_type], bytes(&h(&s["args"]))])
}

fn cell_output(o: &Value) -> Vec<u8> {
    let type_ = if o["type"].is_null() { vec![] } else { script(&o["type"]) };
    table(vec![u64(&o["capacity"]).to_vec(), script(&o["lock"]), type_])
}

fn out_point(p: &Value) -> Vec<u8> {
    [h(&p["tx_hash"]), u32(&p["index"]).to_vec()].concat()
}

fn list(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("not an array: {v}"))
}

fn raw_tx(t: &Value) -> Vec<u8> {
    let dep_type = |d: &Value| match text(&d["dep_type"]) {
        "code" => 0,
        "dep_group" => 1,
        other => panic!("dep type {other}"),
    };
    table(vec![
        u32(&t["version"]).to_vec(),
        fixvec(list(&t["cell_deps"]).iter().map(|d| [out_point(&d["out_point"]), vec![dep_type(d)]].concat()).collect()),
        fixvec(list(&t["header_deps"]).iter().map(h).collect()),
        fixvec(list(&t["inputs"]).iter().map(|i| [u64(&i["since"]).to_vec(), out_point(&i["previous_output"])].concat()).collect()),
        table(list(&t["outputs"]).iter().map(cell_output).collect()),
        table(list(&t["outputs_data"]).iter().map(|d| bytes(&h(d))).collect()),
    ])
}

fn tx_bytes(t: &Value) -> Vec<u8> {
    let raw = raw_tx(t);
    assert_eq!(hash(&raw), text(&t["hash"]), "transaction hash");
    table(vec![raw, table(list(&t["witnesses"]).iter().map(|w| bytes(&h(w))).collect())])
}

fn header_bytes(x: &Value) -> Vec<u8> {
    let mut b = [u32(&x["version"]), u32(&x["compact_target"])].concat();
    for field in ["timestamp", "number", "epoch"] {
        b.extend(u64(&x[field]));
    }
    for field in ["parent_hash", "transactions_root", "proposals_hash", "extra_hash", "dao"] {
        b.extend(h(&x[field]));
    }
    b.extend(int(&x["nonce"]).to_le_bytes());
    assert_eq!(hash(&b), text(&x["hash"]), "header hash");
    b
}

fn script_hash(s: &Value) -> String {
    hash(&script(s))
}

/// The answer to one JSON-RPC batch, or None when the request or the answer fails.
fn post(url: &str, body: &[u8]) -> Option<Vec<Value>> {
    let mut child = Command::new("curl")
        .args(["-sS", "--fail", "--max-time", "120", "-H", "content-type: application/json", "--data-binary", "@-", url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("run curl");
    child.stdin.take().unwrap().write_all(body).expect("write request");
    let out = child.wait_with_output().expect("wait for curl");
    out.status.success().then(|| serde_json::from_slice(&out.stdout).ok()).flatten()
}

struct Rpc {
    url: String,
    calls: AtomicUsize,
}

impl Rpc {
    fn batch(&self, calls: &[(&str, Value)]) -> Vec<Value> {
        let body: Vec<Value> = calls.iter().enumerate().map(|(i, (m, p))| json!({"id": i, "jsonrpc": "2.0", "method": m, "params": p})).collect();
        let body = serde_json::to_vec(&body).unwrap();
        // Six tries, backing off between them, for a transient network failure or rate limit. Each
        // try reads a fresh answer: after a cut-off answer, curl's own retry appends the next one to it.
        // An answer whose entry count differs from the calls counts as failed, so no call drops out silently.
        let mut res = (0..6)
            .find_map(|attempt| {
                if attempt > 0 {
                    std::thread::sleep(Duration::from_secs(1 << (attempt - 1)));
                }
                post(&self.url, &body).filter(|answers| answers.len() == calls.len())
            })
            .unwrap_or_else(|| panic!("no JSON-RPC batch answer for {:?}", calls[0]));
        res.sort_by_key(|x| x["id"].as_u64());
        self.calls.fetch_add(calls.len(), Ordering::Relaxed);
        res.into_iter()
            .map(|mut x| {
                assert!(x["error"].is_null(), "{:?}: {}", calls[x["id"].as_u64().unwrap() as usize], x["error"]);
                x["result"].take()
            })
            .collect()
    }

    fn call(&self, m: &str, p: Value) -> Value {
        self.batch(&[(m, p)]).remove(0)
    }

    /// The calls in batches of 40, four batches at a time, answers in call order.
    fn many(&self, calls: Vec<(&str, Value)>) -> Vec<Value> {
        let batches: Vec<_> = calls.chunks(40).collect();
        let mut out = Vec::new();
        for group in batches.chunks(4) {
            std::thread::scope(|s| {
                let running: Vec<_> = group.iter().map(|b| s.spawn(|| self.batch(b))).collect();
                out.extend(running.into_iter().flat_map(|r| r.join().unwrap()));
            });
        }
        out
    }

    fn get_transactions(&self, hashes: impl IntoIterator<Item = String>) -> Vec<Value> {
        self.many(hashes.into_iter().map(|x| ("get_transaction", json!([x]))).collect())
    }
}

fn scripts(release: &str) -> Map<String, Value> {
    let code_hash = |name: &str| hash(&std::fs::read(format!("{release}/{name}")).expect("read binary"));
    let data1 = |name: &str| json!({"code_hash": code_hash(name), "hash_type": "data1", "args": "0x"});
    let ickb = data1("ickb_logic");
    assert_eq!(code_hash("xudt"), XUDT_CODE_HASH);
    let xudt = json!({"code_hash": XUDT_CODE_HASH, "hash_type": "data1", "args": script_hash(&ickb) + "00000080"});
    let dao = json!({"code_hash": DAO_CODE_HASH, "hash_type": "type", "args": "0x"});
    let mut s = Map::new();
    for (name, v) in [("ickb_logic", ickb), ("owned_owner", data1("owned_owner")), ("limit_order", data1("limit_order")), ("ickb_xudt", xudt), ("dao", dao)] {
        s.insert(name.into(), v);
    }
    s
}

fn pinned_tip(rpc: &Rpc) -> (u64, String) {
    let r = rpc.batch(&[("get_tip_header", json!([])), ("get_indexer_tip", json!([]))]);
    assert_eq!(r[0]["hash"], r[1]["block_hash"], "indexer behind node tip; retry");
    (int(&r[0]["number"]) as u64, text(&r[0]["hash"]).to_string())
}

/// Every page of an indexer query, as `method` returns it for `key`.
fn pages(rpc: &Rpc, method: &str, key: &Value, total: &mut usize) -> Vec<Value> {
    let (mut after, mut out) = (Value::Null, Vec::new());
    loop {
        let mut page = rpc.call(method, json!([key, "asc", format!("{PAGE:#x}"), after]));
        let objects = page["objects"].as_array_mut().expect("objects").split_off(0);
        *total += objects.len();
        assert!(*total <= TOTAL_BOUND, "total bound exceeded");
        let last = objects.len() < PAGE as usize;
        out.extend(objects);
        if last {
            return out;
        }
        after = page["last_cursor"].take();
        assert!(!after.is_null(), "a full page without last_cursor");
    }
}

fn search_key(script: &Value, role: &str, tip: u64) -> Value {
    json!({"script": script, "script_type": role, "script_search_mode": "exact", "filter": {"block_range": ["0x0", format!("{:#x}", tip + 1)]}})
}

fn write(path: String, text: &[u8]) {
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
}

/// The standard output of a command that must succeed, trimmed.
fn output(cmd: &str, args: &[&str]) -> String {
    let out = Command::new(cmd).args(args).output().unwrap_or_else(|e| panic!("run {cmd}: {e}"));
    assert!(out.status.success(), "{cmd} {args:?} failed");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn fetch(release: &str, out: &str, url: &str) {
    let rpc = Rpc { url: url.into(), calls: AtomicUsize::new(0) };
    let s = scripts(release);
    let (tip, tip_hash) = pinned_tip(&rpc);
    let (mut roles, mut total, mut counts) = (BTreeMap::<String, BTreeSet<String>>::new(), 0, Map::new());
    for name in ["ickb_logic", "owned_owner", "limit_order", "ickb_xudt"] {
        for role in ["lock", "type"] {
            let mut key = search_key(&s[name], role, tip);
            key["group_by_transaction"] = json!(true);
            let found = pages(&rpc, "get_transactions", &key, &mut total);
            for o in &found {
                roles.entry(text(&o["tx_hash"]).into()).or_default().insert(format!("{name}:{role}"));
            }
            counts.insert(format!("{name}:{role}"), json!(found.len()));
            eprintln!("{name}:{role} {}", found.len());
        }
    }
    eprintln!("{} distinct transactions", roles.len());
    let mut txs = BTreeMap::new();
    for r in rpc.get_transactions(roles.keys().cloned()) {
        assert_eq!(r["tx_status"]["status"], "committed");
        txs.insert(text(&r["transaction"]["hash"]).to_string(), r);
    }
    let inputs = || txs.values().flat_map(|r| list(&r["transaction"]["inputs"]));
    let prev_hashes: BTreeSet<String> =
        inputs().map(|i| text(&i["previous_output"]["tx_hash"]).to_string()).filter(|x| !txs.contains_key(x)).collect();
    let mut prevs = txs.clone();
    for r in rpc.get_transactions(prev_hashes.iter().cloned()) {
        prevs.insert(text(&r["transaction"]["hash"]).to_string(), r);
    }
    // Cell deps, then the members of the dep groups among them, each as its molecule OutPoint. A
    // spent dep is still served by get_transaction, so the replay sees the code cells each
    // transaction ran against.
    let deps: Vec<&Value> = txs.values().flat_map(|r| list(&r["transaction"]["cell_deps"])).collect();
    let mut dep_points: BTreeSet<Vec<u8>> = deps.iter().map(|d| out_point(&d["out_point"])).collect();
    let mut dep_txs = BTreeMap::new();
    let fetch_dep_txs = |dep_points: &BTreeSet<Vec<u8>>, dep_txs: &mut BTreeMap<String, Value>| {
        let missing: BTreeSet<String> = dep_points.iter().map(|p| format!("0x{}", hex(&p[..32]))).filter(|x| !dep_txs.contains_key(x)).collect();
        for r in rpc.get_transactions(missing) {
            tx_bytes(&r["transaction"]); // proves the dep cell bytes come from the committed transaction
            dep_txs.insert(text(&r["transaction"]["hash"]).to_string(), r);
        }
    };
    let dep_cell = |dep_txs: &BTreeMap<String, Value>, p: &[u8]| {
        let t = &dep_txs[&format!("0x{}", hex(&p[..32]))]["transaction"];
        let k = le(&p[32..]) as usize;
        (cell_output(&t["outputs"][k]), h(&t["outputs_data"][k]))
    };
    fetch_dep_txs(&dep_points, &mut dep_txs);
    let groups: BTreeSet<Vec<u8>> = deps.iter().filter(|d| d["dep_type"] == "dep_group").map(|d| out_point(&d["out_point"])).collect();
    for group in groups {
        let members = dep_cell(&dep_txs, &group).1;
        let count = le(&members[..4]) as usize;
        assert_eq!(members.len(), 4 + 36 * count, "dep group data is not an OutPointVec");
        dep_points.extend(members[4..].chunks(36).map(<[u8]>::to_vec));
    }
    fetch_dep_txs(&dep_points, &mut dep_txs);
    let dep_lines: Vec<String> = dep_points
        .iter()
        .map(|p| {
            let (o, data) = dep_cell(&dep_txs, p);
            format!("dep {} {} {}", hex(p), hex(&o), hex(&data))
        })
        .collect();
    let mut header_hashes = BTreeSet::new();
    for r in txs.values() {
        header_hashes.extend(list(&r["transaction"]["header_deps"]).iter().map(|x| text(x).to_string()));
    }
    header_hashes.extend(inputs().map(|i| text(&prevs[text(&i["previous_output"]["tx_hash"])]["tx_status"]["block_hash"]).to_string()));
    let headers: Map<String, Value> = rpc
        .many(header_hashes.iter().map(|x| ("get_header", json!([x]))).collect())
        .into_iter()
        .map(|x| (text(&x["hash"]).to_string(), x))
        .collect();
    assert!(header_hashes.iter().all(|x| headers.contains_key(x)));
    let mut order: Vec<&Value> = txs.values().collect();
    order.sort_by_key(|r| (int(&r["tx_status"]["block_number"]), text(&r["transaction"]["hash"])));
    let (mut lines, mut cells) = (Vec::new(), 0);
    for r in order {
        let t = &r["transaction"];
        let tx_roles: Vec<&str> = roles[text(&t["hash"])].iter().map(String::as_str).collect();
        lines.push(format!("tx {} {} {} {}", text(&t["hash"]), text(&r["tx_status"]["block_hash"]), tx_roles.join(" "), hex(&tx_bytes(t))));
        for i in list(&t["inputs"]) {
            let p = &i["previous_output"];
            let pr = &prevs[text(&p["tx_hash"])];
            tx_bytes(&pr["transaction"]); // proves the cell bytes below come from the committed transaction
            let k = int(&p["index"]) as usize;
            let o = &pr["transaction"]["outputs"][k];
            let data = &text(&pr["transaction"]["outputs_data"][k])[2..];
            lines.push(format!("cell {} {} {} {data}", hex(&out_point(p)), text(&pr["tx_status"]["block_hash"]), hex(&cell_output(o))));
            cells += 1;
        }
        lines.push("end".into());
    }
    let head_lines = header_hashes.iter().map(|x| format!("header {}", hex(&header_bytes(&headers[x]))));
    let meta_s: Map<String, Value> = s.iter().map(|(k, v)| (k.clone(), json!(script_hash(v)))).collect();
    let first = format!("scripts {}", ["ickb_logic", "ickb_xudt", "owned_owner", "limit_order", "dao"].map(|k| text(&meta_s[k]).to_string()).join(" "));
    let fixtures: String = [first].into_iter().chain(dep_lines.iter().cloned()).chain(head_lines).chain(lines).map(|l| l + "\n").collect();
    write(format!("{out}/fixtures.txt"), fixtures.as_bytes());
    let only_prevs: Map<String, Value> = prevs.into_iter().filter(|(k, _)| !txs.contains_key(k)).collect();
    let raw = json!({"txs": txs, "prevs": only_prevs, "deps": dep_txs, "headers": headers});
    write(format!("{out}/raw.json"), &serde_json::to_vec(&raw).unwrap());
    let mut meta = json!({"rpc": url, "tip_number": tip, "tip_hash": tip_hash, "fetched_at": output("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]),
        "indexer_entries_per_role": counts, "transactions": txs.len(), "previous_transactions": prev_hashes.len(),
        "input_cells": cells, "dep_cells": dep_lines.len(), "headers": header_hashes.len(), "scripts": s, "script_hashes": meta_s,
        "rpc_calls": rpc.calls.load(Ordering::Relaxed)});
    for name in ["fixtures.txt", "raw.json"] {
        let digest = output("sha256sum", &[&format!("{out}/{name}")]);
        meta[format!("sha256:{name}")] = json!(digest.split(' ').next().unwrap());
    }
    write(format!("{out}/meta.json"), &serde_json::to_vec_pretty(&meta).unwrap());
    meta.as_object_mut().unwrap().remove("scripts");
    println!("{}", serde_json::to_string_pretty(&meta).unwrap());
}

fn occupied(o: &Value, data: &Value) -> u128 {
    let size = |s: &Value| 32 + 1 + h(&s["args"]).len() as u128;
    let type_ = if o["type"].is_null() { 0 } else { size(&o["type"]) };
    (8 + size(&o["lock"]) + type_ + h(data).len() as u128) * CKB
}

fn soft_capped(amount: u128, ar: u128) -> u128 {
    let (raw, cap) = (amount * GENESIS_AR / ar, 100_000 * CKB);
    if raw > cap {
        raw - (raw - cap) / 10
    } else {
        raw
    }
}

fn ar_of(header: &Value) -> u128 {
    le(&h(&header["dao"])[8..16])
}

fn supply(release: &str, out: &str, url: &str) {
    let rpc = Rpc { url: url.into(), calls: AtomicUsize::new(0) };
    let s = scripts(release);
    let (tip, tip_hash) = pinned_tip(&rpc);
    let (dao_hash, ickb_hash) = (script_hash(&s["dao"]), script_hash(&s["ickb_logic"]));
    let live = |name: &str, role: &str| pages(&rpc, "get_cells", &search_key(&s[name], role, tip), &mut 0);
    let (udt, locked, receipts) = (live("ickb_xudt", "type"), live("ickb_logic", "lock"), live("ickb_logic", "type"));
    let (tip_after, tip_after_hash) = pinned_tip(&rpc);
    // The three scans read the live set at different moments; only an unchanged tip makes them
    // one state, so the totals below describe exactly that block.
    if tip_after_hash != tip_hash {
        eprintln!("tip moved from {tip} to {tip_after} during the scan; run supply again");
        std::process::exit(1);
    }
    let need: BTreeSet<&str> = locked.iter().chain(&receipts).map(|c| text(&c["block_number"])).collect();
    let headers = rpc.many(need.iter().map(|n| ("get_header_by_number", json!([n]))).collect());
    let blocks: BTreeMap<&str, &Value> = need.iter().copied().zip(&headers).collect();
    let ar = |c: &Value| ar_of(blocks[text(&c["block_number"])]);
    let has_type = |c: &Value, hash: &str| !c["output"]["type"].is_null() && script_hash(&c["output"]["type"]) == hash;
    let total_udt: u128 = udt.iter().map(|c| h(&c["output_data"])).map(|d| le(&d[..d.len().min(16)])).sum();
    let (mut deposits, mut n_dep, mut other_locked) = (0, 0, 0);
    for c in &locked {
        if has_type(c, &dao_hash) && c["output_data"] == format!("0x{}", "00".repeat(8)) {
            let unused = int(&c["output"]["capacity"]) - occupied(&c["output"], &c["output_data"]);
            deposits += soft_capped(unused, ar(c));
            n_dep += 1;
        } else {
            other_locked += 1;
        }
    }
    let (mut rec, mut n_rec, mut bad_rec) = (0, 0, 0);
    for c in &receipts {
        let d = h(&c["output_data"]);
        if has_type(c, &ickb_hash) && d.len() >= 12 {
            rec += le(&d[..4]) * soft_capped(le(&d[4..12]), ar(c));
            n_rec += 1;
        } else {
            bad_rec += 1;
        }
    }
    let (backing, supply) = (deposits + rec, total_udt);
    let res = json!({"rpc": url, "tip_number": tip, "tip_hash": tip_hash, "tip_after_scan": tip_after, "tip_after_hash": tip_after_hash,
        "xudt_cells": udt.len(), "supply": supply, "deposit_cells": n_dep, "deposits_value": deposits,
        "ickb_locked_non_deposit_cells": other_locked, "receipt_cells": n_rec, "receipts_value": rec,
        "receipt_cells_malformed": bad_rec, "backing": backing, "gap_backing_minus_supply": backing as i128 - supply as i128,
        "supply_le_backing": supply <= backing,
        // A deposit transaction creates deposits and a receipt of equal value, so a receipt's
        // deposit is already counted: the exact identity is deposits == supply + receipts.
        "deposits_minus_supply_minus_receipts": deposits as i128 - supply as i128 - rec as i128});
    write(format!("{out}/supply.json"), &serde_json::to_vec_pretty(&res).unwrap());
    println!("{}", serde_json::to_string_pretty(&res).unwrap());
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cmd, release, out, rest @ ..] = &args[..] else { panic!("usage: ickb-mainnet fetch|supply <release_dir> <out_dir> [rpc_url]") };
    let url = rest.first().map_or(RPC, String::as_str);
    match cmd.as_str() {
        "fetch" => fetch(release, out, url),
        "supply" => supply(release, out, url),
        other => panic!("unknown command {other}"),
    }
}
