"""Fetch mainnet iCKB transactions as replay fixtures, and check live iCKB supply.

Usage (run with python3 -I):
  ickb_mainnet.py fetch  <release_dir> <out_dir> [rpc_url]
  ickb_mainnet.py supply <release_dir> <out_dir> [rpc_url]

fetch writes <out_dir>/fixtures.txt (one record per line, molecule hex, read by the Rust replay test)
and <out_dir>/meta.json. Every fetched transaction and header is re-serialized here and its hash
checked against the RPC hash, so the molecule encoding is proven before it reaches the fixture.
"""
import hashlib
import json
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor

RPC = "https://mainnet.ckb.dev/rpc"
# mainnet.ckb.dev answers 403 to Python-urllib's default user agent.
UA = "curl/8.9.1"
XUDT_CODE_HASH = "0x50bd8d6680b8b9cf98b73f3c08faf8b2a21914311954118ad6609be6e78a1b95"
DAO_CODE_HASH = "0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e"
PAGE = 1000
TOTAL_BOUND = 1_000_000  # hard cap on indexer entries across all queries
GENESIS_AR = 10_000_000_000_000_000
CKB = 100_000_000
HASH_TYPE = {"data": 0, "type": 1, "data1": 2, "data2": 4}
DEP_TYPE = {"code": 0, "dep_group": 1}


def blake2b(b):
    return hashlib.blake2b(b, digest_size=32, person=b"ckb-default-hash").digest()


def h(s):
    return bytes.fromhex(s[2:])


def u32(n):
    return int(n, 16).to_bytes(4, "little") if isinstance(n, str) else n.to_bytes(4, "little")


def u64(n):
    return int(n, 16).to_bytes(8, "little") if isinstance(n, str) else n.to_bytes(8, "little")


def fixvec(items):
    return len(items).to_bytes(4, "little") + b"".join(items)


def table(fields):
    head = 4 + 4 * len(fields)
    offsets, off = [], head
    for f in fields:
        offsets.append(off)
        off += len(f)
    return off.to_bytes(4, "little") + b"".join(o.to_bytes(4, "little") for o in offsets) + b"".join(fields)


def script(s):
    return table([h(s["code_hash"]), bytes([HASH_TYPE[s["hash_type"]]]), fixvec([bytes([x]) for x in h(s["args"])])])


def cell_output(o):
    return table([u64(o["capacity"]), script(o["lock"]), script(o["type"]) if o["type"] else b""])


def out_point(p):
    return h(p["tx_hash"]) + u32(p["index"])


def raw_tx(t):
    return table([
        u32(t["version"]),
        fixvec([out_point(d["out_point"]) + bytes([DEP_TYPE[d["dep_type"]]]) for d in t["cell_deps"]]),
        fixvec([h(x) for x in t["header_deps"]]),
        fixvec([u64(i["since"]) + out_point(i["previous_output"]) for i in t["inputs"]]),
        table([cell_output(o) for o in t["outputs"]]),
        table([fixvec([bytes([x]) for x in h(d)]) for d in t["outputs_data"]]),
    ])


def tx_bytes(t):
    raw = raw_tx(t)
    assert "0x" + blake2b(raw).hex() == t["hash"], t["hash"]
    return table([raw, table([fixvec([bytes([x]) for x in h(w)]) for w in t["witnesses"]])])


def header_bytes(x):
    b = (u32(x["version"]) + u32(x["compact_target"]) + u64(x["timestamp"]) + u64(x["number"]) + u64(x["epoch"])
         + h(x["parent_hash"]) + h(x["transactions_root"]) + h(x["proposals_hash"]) + h(x["extra_hash"])
         + h(x["dao"]) + int(x["nonce"], 16).to_bytes(16, "little"))
    assert "0x" + blake2b(b).hex() == x["hash"], x["hash"]
    return b


def script_hash(s):
    return "0x" + blake2b(script(s)).hex()


class Rpc:
    def __init__(self, url):
        self.url, self.calls = url, 0

    def batch(self, calls):
        body = json.dumps([{"id": i, "jsonrpc": "2.0", "method": m, "params": p} for i, (m, p) in enumerate(calls)]).encode()
        for attempt in range(6):
            try:
                req = urllib.request.Request(self.url, body, {"content-type": "application/json", "user-agent": UA})
                with urllib.request.urlopen(req, timeout=120) as r:
                    res = sorted(json.load(r), key=lambda x: x["id"])
                break
            except Exception as e:  # transient network or rate limit: back off and retry
                if attempt == 5:
                    raise
                time.sleep(2 ** attempt)
        self.calls += len(calls)
        for x in res:
            if "error" in x and x["error"]:
                raise RuntimeError(f"{calls[x['id']]}: {x['error']}")
        return [x["result"] for x in res]

    def call(self, m, *p):
        return self.batch([(m, list(p))])[0]

    def many(self, calls, size=40, workers=4):
        chunks = [calls[i:i + size] for i in range(0, len(calls), size)]
        with ThreadPoolExecutor(workers) as ex:
            return [r for part in ex.map(self.batch, chunks) for r in part]


def scripts(release):
    def data1(name, args="0x"):
        with open(f"{release}/{name}", "rb") as f:
            return {"code_hash": "0x" + blake2b(f.read()).hex(), "hash_type": "data1", "args": args}
    ickb = data1("ickb_logic")
    xudt = {"code_hash": XUDT_CODE_HASH, "hash_type": "data1", "args": script_hash(ickb) + "00000080"}
    with open(f"{release}/xudt", "rb") as f:
        assert "0x" + blake2b(f.read()).hex() == XUDT_CODE_HASH
    dao = {"code_hash": DAO_CODE_HASH, "hash_type": "type", "args": "0x"}
    return {"ickb_logic": ickb, "owned_owner": data1("owned_owner"), "limit_order": data1("limit_order"), "ickb_xudt": xudt, "dao": dao}


def pinned_tip(rpc):
    tip, itip = rpc.batch([("get_tip_header", []), ("get_indexer_tip", [])])
    assert tip["hash"] == itip["block_hash"], "indexer behind node tip; retry"
    return int(tip["number"], 16), tip["hash"]


def fetch(release, out, url):
    rpc = Rpc(url)
    s = scripts(release)
    tip, tip_hash = pinned_tip(rpc)
    roles, total = {}, 0
    counts = {}
    for name in ["ickb_logic", "owned_owner", "limit_order", "ickb_xudt"]:
        for role in ["lock", "type"]:
            key = {"script": s[name], "script_type": role, "script_search_mode": "exact",
                   "filter": {"block_range": [hex(0), hex(tip + 1)]}, "group_by_transaction": True}
            after, n = None, 0
            while True:
                page = rpc.call("get_transactions", key, "asc", hex(PAGE), after)
                for o in page["objects"]:
                    roles.setdefault(o["tx_hash"], set()).add(f"{name}:{role}")
                n += len(page["objects"])
                total += len(page["objects"])
                assert total <= TOTAL_BOUND, "total bound exceeded"
                if len(page["objects"]) < PAGE:
                    break
                after = page["last_cursor"]
            counts[f"{name}:{role}"] = n
            print(f"{name}:{role} {n}", file=sys.stderr)
    hashes = sorted(roles)
    print(f"{len(hashes)} distinct transactions", file=sys.stderr)
    txs = {}
    for r in rpc.many([("get_transaction", [x]) for x in hashes]):
        assert r["tx_status"]["status"] == "committed"
        txs[r["transaction"]["hash"]] = r
    prev_hashes = sorted({i["previous_output"]["tx_hash"] for r in txs.values() for i in r["transaction"]["inputs"]} - set(txs))
    prevs = dict(txs)
    for r in rpc.many([("get_transaction", [x]) for x in prev_hashes]):
        prevs[r["transaction"]["hash"]] = r
    header_hashes = set()
    for r in txs.values():
        header_hashes.update(r["transaction"]["header_deps"])
        for i in r["transaction"]["inputs"]:
            header_hashes.add(prevs[i["previous_output"]["tx_hash"]]["tx_status"]["block_hash"])
    header_hashes = sorted(header_hashes)
    headers = {x["hash"]: x for x in rpc.many([("get_header", [x]) for x in header_hashes])}
    assert all(headers.get(x) for x in header_hashes)
    order = sorted(txs.values(), key=lambda r: (int(r["tx_status"]["block_number"], 16), r["transaction"]["hash"]))
    lines, cells = [], 0
    for r in order:
        t = r["transaction"]
        lines.append(f"tx {t['hash']} {r['tx_status']['block_hash']} {' '.join(sorted(roles[t['hash']]))} {tx_bytes(t).hex()}")
        for i in t["inputs"]:
            p = i["previous_output"]
            pr = prevs[p["tx_hash"]]
            tx_bytes(pr["transaction"])  # proves the cell bytes below come from the committed transaction
            k = int(p["index"], 16)
            o = pr["transaction"]["outputs"][k]
            lines.append(f"cell {out_point(p).hex()} {pr['tx_status']['block_hash']} {cell_output(o).hex()} {pr['transaction']['outputs_data'][k][2:]}")
            cells += 1
        lines.append("end")
    head_lines = [f"header {header_bytes(headers[x]).hex()}" for x in header_hashes]
    meta_s = {k: script_hash(v) for k, v in s.items()}
    text = "".join(l + "\n" for l in [f"scripts {meta_s['ickb_logic']} {meta_s['ickb_xudt']} {meta_s['owned_owner']} {meta_s['limit_order']} {meta_s['dao']}"] + head_lines + lines)
    with open(f"{out}/fixtures.txt", "w") as f:
        f.write(text)
    raw = {"txs": txs, "prevs": {k: v for k, v in prevs.items() if k not in txs}, "headers": headers}
    with open(f"{out}/raw.json", "w") as f:
        json.dump(raw, f, sort_keys=True)
    meta = {"rpc": url, "tip_number": tip, "tip_hash": tip_hash, "fetched_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "indexer_entries_per_role": counts, "transactions": len(txs), "previous_transactions": len(prev_hashes),
            "input_cells": cells, "headers": len(header_hashes), "scripts": s, "script_hashes": meta_s, "rpc_calls": rpc.calls}
    for name in ["fixtures.txt", "raw.json"]:
        with open(f"{out}/{name}", "rb") as f:
            meta[f"sha256:{name}"] = hashlib.sha256(f.read()).hexdigest()
    with open(f"{out}/meta.json", "w") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
    print(json.dumps({k: v for k, v in meta.items() if k != "scripts"}, indent=1))


def occupied(o, data_hex):
    def ssize(x):
        return 32 + 1 + len(h(x["args"]))
    return (8 + ssize(o["lock"]) + (ssize(o["type"]) if o["type"] else 0) + len(h(data_hex))) * CKB


def soft_capped(amount, ar):
    raw = amount * GENESIS_AR // ar
    cap = 100_000 * CKB
    return raw - (raw - cap) // 10 if raw > cap else raw


def ar_of(header):
    return int.from_bytes(h(header["dao"])[8:16], "little")


def live_cells(rpc, sc, role, tip):
    key = {"script": sc, "script_type": role, "script_search_mode": "exact", "filter": {"block_range": [hex(0), hex(tip + 1)]}}
    after, out = None, []
    while True:
        page = rpc.call("get_cells", key, "asc", hex(PAGE), after)
        out += page["objects"]
        assert len(out) <= TOTAL_BOUND
        if len(page["objects"]) < PAGE:
            return out
        after = page["last_cursor"]


def supply(release, out, url):
    rpc = Rpc(url)
    s = scripts(release)
    tip, tip_hash = pinned_tip(rpc)
    dao_hash, ickb_hash = script_hash(s["dao"]), script_hash(s["ickb_logic"])
    udt = live_cells(rpc, s["ickb_xudt"], "type", tip)
    locked = live_cells(rpc, s["ickb_logic"], "lock", tip)
    receipts = live_cells(rpc, s["ickb_logic"], "type", tip)
    tip_after, tip_after_hash = pinned_tip(rpc)
    # The three scans read the live set at different moments; only an unchanged tip makes them
    # one state, so the totals below describe exactly that block.
    if tip_after_hash != tip_hash:
        sys.exit(f"tip moved from {tip} to {tip_after} during the scan; run supply again")
    need = sorted({c["block_number"] for c in locked + receipts})
    blocks = dict(zip(need, rpc.many([("get_header_by_number", [n]) for n in need])))
    total_udt = sum(int.from_bytes(h(c["output_data"])[:16], "little") for c in udt)
    deposits, n_dep, other_locked = 0, 0, 0
    for c in locked:
        o = c["output"]
        if o["type"] and script_hash(o["type"]) == dao_hash and c["output_data"] == "0x" + "00" * 8:
            unused = int(o["capacity"], 16) - occupied(o, c["output_data"])
            deposits += soft_capped(unused, ar_of(blocks[c["block_number"]]))
            n_dep += 1
        else:
            other_locked += 1
    rec, n_rec, bad_rec = 0, 0, 0
    for c in receipts:
        d = h(c["output_data"])
        if c["output"]["type"] and script_hash(c["output"]["type"]) == ickb_hash and len(d) >= 12:
            q, a = int.from_bytes(d[:4], "little"), int.from_bytes(d[4:12], "little")
            rec += q * soft_capped(a, ar_of(blocks[c["block_number"]]))
            n_rec += 1
        else:
            bad_rec += 1
    res = {"rpc": url, "tip_number": tip, "tip_hash": tip_hash, "tip_after_scan": tip_after, "tip_after_hash": tip_after_hash,
           "xudt_cells": len(udt), "supply": total_udt, "deposit_cells": n_dep, "deposits_value": deposits,
           "ickb_locked_non_deposit_cells": other_locked, "receipt_cells": n_rec, "receipts_value": rec,
           "receipt_cells_malformed": bad_rec, "backing": deposits + rec, "gap_backing_minus_supply": deposits + rec - total_udt,
           "supply_le_backing": total_udt <= deposits + rec,
           # A deposit transaction creates deposits and a receipt of equal value, so a receipt's
           # deposit is already counted: the exact identity is deposits == supply + receipts.
           "deposits_minus_supply_minus_receipts": deposits - total_udt - rec}
    with open(f"{out}/supply.json", "w") as f:
        json.dump(res, f, indent=1)
    print(json.dumps(res, indent=1))


if __name__ == "__main__":
    cmd, release, out = sys.argv[1:4]
    url = sys.argv[4] if len(sys.argv) > 4 else RPC
    {"fetch": fetch, "supply": supply}[cmd](release, out, url)
