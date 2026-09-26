fn deposit_to_ickb(index: usize, source: Source, amount: u64) -> Result<u128, Error> {
    let amount = u128::from(amount);
    let ar_0 = GENESIS_ACCUMULATED_RATE;
    let ar_m = u128::from(extract_accumulated_rate(index, source)?);

    let ickb_amount = amount * ar_0 / ar_m;

    // Apply a 10% discount for the amount exceeding the soft iCKB cap per deposit
    if ickb_amount > ICKB_SOFT_CAP_PER_DEPOSIT {
        return Ok(ickb_amount - (ickb_amount - ICKB_SOFT_CAP_PER_DEPOSIT) / 10);
    }

    Ok(ickb_amount)
}
