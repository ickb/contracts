fn validate(i: Data, o: Data) -> Result<(), Error> {
    if i.info != o.info {
        return Err(Error::DifferentInfo);
    }

    let (is_ckb_to_udt, Ratio { ckb_mul, udt_mul }, ckb_min_match) = match (
        i.info.ckb_to_udt,
        i.ckb > o.ckb,
        i.info.udt_to_ckb,
        i.udt > o.udt,
    ) {
        (Some(ratio), true, _, false) => (true, ratio, i.info.ckb_min_match),
        (_, false, Some(ratio), true) => (false, ratio, i.info.ckb_min_match),
        _ => return Err(Error::InvalidMatch),
    };

    // Check that limit order does not lose value
    if i.ckb * ckb_mul + i.udt * udt_mul > o.ckb * ckb_mul + o.udt * udt_mul {
        return Err(Error::DecreasingValue);
    }

    // Validate limit order match
    if is_ckb_to_udt {
        // CKB -> UDT
        // Check that an already fulfilled order is not modified
        if i.ckb_unoccupied.is_zero() {
            return Err(Error::AttemptToChangeFulfilled);
        }

        // DOS prevention: disallow partial match lower than the equivalent of ckb_min_match
        if !o.ckb_unoccupied.is_zero() && i.ckb < o.ckb + ckb_min_match {
            return Err(Error::InsufficientMatch);
        }
    } else {
        // UDT -> CKB
        // Check that an already fulfilled order is not modified
        if i.udt.is_zero() {
            return Err(Error::AttemptToChangeFulfilled);
        }

        // DOS prevention: disallow partial match lower than the equivalent of ckb_min_match
        if !o.udt.is_zero() && i.udt * udt_mul < o.udt * udt_mul + ckb_min_match * ckb_mul {
            return Err(Error::InsufficientMatch);
        }
    }

    Ok(())
}
