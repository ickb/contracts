#[derive(Clone, Copy, PartialEq)]
struct Data {
    ckb: C256,
    udt: C256,
    ckb_unoccupied: C256,
    info: Info,
}

#[derive(Clone, Copy, PartialEq)]
struct Info {
    udt_hash: [u8; 32],
    ckb_to_udt: Option<Ratio>,
    udt_to_ckb: Option<Ratio>,
    ckb_min_match: C256,
}

#[derive(Clone, Copy, PartialEq)]
struct Ratio {
    ckb_mul: C256,
    udt_mul: C256,
}
