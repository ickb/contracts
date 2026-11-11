use super::*;

// Replays committed mainnet transactions that touch the iCKB scripts through the fuzz oracle.
// Testnet runs the same deployment (same script hashes), so a testnet RPC URL works too.
// Each was accepted on chain, so a violation means the oracle mis-models a legitimate shape or
// the deployed scripts accepted a break. Each is also executed here, every script group against
// the cell deps the chain used: a rejection means this harness runs scripts differently from the
// node that accepted the transaction. Fixtures are fetched separately and read from
// `ICKB_MAINNET_FIXTURES`, so the test needs no network and stays out of the default suite:
//   cargo run --locked --release --manifest-path mainnet/Cargo.toml -- fetch ../build/release <out_dir> [rpc_url]
//   ICKB_MAINNET_FIXTURES=<out_dir>/fixtures.txt cargo test --locked -- --ignored mainnet
// both from scripts/tests.
//
// Fixture format, one record per line, hex without 0x:
//   scripts <ickb_logic> <ickb_xudt> <owned_owner> <limit_order> <dao>   (mainnet script hashes)
//   dep <molecule OutPoint> <molecule CellOutput> <data>                 (every cell dep and dep group member)
//   header <molecule Header>
//   tx <tx hash> <block hash> <roles...> <molecule Transaction>
//   cell <molecule OutPoint> <block hash> <molecule CellOutput> <data>   (one per input, in order)
//   end

fn hex32(s: &str) -> Byte32 {
    Byte32::from_slice(&hex::decode(s.trim_start_matches("0x")).expect("hex")).expect("32 bytes")
}

#[test]
#[ignore = "needs fetched mainnet fixtures in ICKB_MAINNET_FIXTURES"]
fn mainnet_transactions_keep_protocol_invariants() {
    let path = std::env::var("ICKB_MAINNET_FIXTURES").expect("ICKB_MAINNET_FIXTURES names the fixture file");
    let text = std::fs::read_to_string(&path).expect("read fixtures");
    let mut context = Context::default();
    let (mut replayed, mut failures) = (0usize, Vec::new());
    let mut tx: Option<(TransactionView, String)> = None;
    for line in text.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        match f[0] {
            "scripts" => {
                // The oracle names scripts by the hashes every test context deploys; they must be the chain's.
                let expected: Vec<Byte32> = f[1..].iter().map(|s| hex32(s)).collect();
                let h = deployed();
                let actual = [&h.ickb, &h.xudt, &h.owned_owner, &h.limit_order, &h.dao];
                assert_eq!(actual.map(|h| h.clone()).to_vec(), expected, "deployed script hashes differ from the chain's");
            }
            "dep" => {
                let out_point = OutPoint::from_slice(&hex::decode(f[1]).expect("hex")).expect("out point");
                let output = CellOutput::from_slice(&hex::decode(f[2]).expect("hex")).expect("cell output");
                context.create_cell_with_out_point(out_point, output, Bytes::from(hex::decode(f[3]).expect("hex")));
            }
            "header" => {
                let header = Header::from_slice(&hex::decode(f[1]).expect("hex")).expect("header").into_view();
                context.insert_header(header);
            }
            "tx" => {
                let view = Transaction::from_slice(&hex::decode(f[f.len() - 1]).expect("hex")).expect("tx").into_view();
                assert_eq!(view.hash(), hex32(f[1]), "fixture transaction hash");
                tx = Some((view, f[3..f.len() - 1].join(" ")));
            }
            "cell" => {
                let out_point = OutPoint::from_slice(&hex::decode(f[1]).expect("hex")).expect("out point");
                let output = CellOutput::from_slice(&hex::decode(f[3]).expect("hex")).expect("cell output");
                let data = Bytes::from(hex::decode(f[4]).expect("hex"));
                context.create_cell_with_out_point(out_point.clone(), output, data);
                context.link_cell_with_block(out_point, hex32(f[2]), 0);
            }
            "end" => {
                let (view, roles) = tx.take().expect("end without tx");
                let mut violations = check_invariants(&context, &view);
                // A committed transaction passed the node, so a rejection here is a harness bug.
                violations.extend(context.node_rejection(&view).map(|r| format!("the node check rejects it: {r}")));
                // The chain ran these scripts under its block cycle limit and accepted them.
                violations.extend(context.verify_tx(&view, mainnet().max_block_cycles()).err().map(|e| format!("the harness scripts reject it: {e}")));
                if !violations.is_empty() {
                    failures.push(format!("{:#x} ({roles}):\n    {}", view.hash(), violations.join("\n    ")));
                }
                replayed += 1;
            }
            other => panic!("unknown fixture record {other}"),
        }
    }
    println!("replayed {replayed} transactions, {} with violations", failures.len());
    assert!(replayed > 0, "no transactions in {path}");
    assert!(failures.is_empty(), "{} transactions fail the oracle, the node check or the scripts:\n{}", failures.len(), failures.join("\n"));
}
