//! Golden protocol vector generator for the iCKB stack testkit: writes the rows
//! of `vectors.rs` as JSON.
//!
//! Usage: cargo run --release -- <output.json>

mod vectors;

use vectors::{limit_order, DepositRow, LimitRow};

fn json_str(s: &str) -> &str {
    assert!(
        s.chars().all(|c| c.is_ascii() && c != '"' && c != '\\'),
        "string needs JSON escaping: {s}"
    );
    s
}

fn ratio_json(r: Option<(u64, u64)>) -> String {
    match r {
        Some((c, u)) => format!("{{\"ckbMul\": \"{c}\", \"udtMul\": \"{u}\"}}"),
        None => "null".into(),
    }
}

fn side_json((ckb, udt, unoccupied): limit_order::Side) -> String {
    format!("{{\"ckb\": \"{ckb}\", \"udt\": \"{udt}\", \"ckbUnoccupied\": \"{unoccupied}\"}}")
}

fn main() {
    vectors::drift_checks();

    let deposit_json: Vec<String> = vectors::deposit_rows()
        .iter()
        .map(|DepositRow { ar, amount, expected_ickb, note }| {
            format!(
                "    {{\"arDecimal\": \"{ar}\", \"unoccupiedShannons\": \"{amount}\", \"expectedIckb\": \"{expected_ickb}\", \"note\": \"{}\"}}",
                json_str(note)
            )
        })
        .collect();

    let limit_json: Vec<String> = vectors::limit_rows()
        .iter()
        .map(|row| {
            let LimitRow { ckb_to_udt, udt_to_ckb, ckb_min_match_log: log, input, output, verdict, note } = row;
            let verdict = match verdict {
                Ok(()) => "ok".into(),
                Err(e) => format!("{e:?}"),
            };
            format!(
                "    {{\"ckbToUdt\": {}, \"udtToCkb\": {}, \"ckbMinMatchLog\": {log}, \"input\": {}, \"output\": {}, \"verdict\": \"{verdict}\", \"note\": \"{}\"}}",
                ratio_json(*ckb_to_udt),
                ratio_json(*udt_to_ckb),
                side_json(*input),
                side_json(*output),
                json_str(note)
            )
        })
        .collect();

    let out = format!(
        "{{\n  \"depositToIckb\": [\n{}\n  ],\n  \"limitOrderMatch\": [\n{}\n  ]\n}}\n",
        deposit_json.join(",\n"),
        limit_json.join(",\n")
    );

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "protocol_vectors.json".into());
    std::fs::write(&path, out).expect("failed to write output");
    println!(
        "wrote {} depositToIckb rows and {} limitOrderMatch rows to {path}",
        deposit_json.len(),
        limit_json.len()
    );
}
