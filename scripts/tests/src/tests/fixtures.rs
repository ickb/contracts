use super::*;
use ckb_dao_utils::pack_dao_data;
use ckb_system_scripts::BUNDLED_CELL;

pub(super) fn deployed_cell(context: &mut Context, data: Bytes) -> OutPoint {
    context.deploy_cell(data)
}

pub(super) fn bundled_cell(context: &mut Context, path: &str) -> OutPoint {
    let data = Bytes::from(BUNDLED_CELL.get(path).expect("bundled cell").into_owned());
    deployed_cell(context, data)
}

// Common script constructors used across contract suites.
pub(super) fn always_success_script(context: &mut Context, args: Bytes) -> Script {
    let out_point = deployed_cell(context, ALWAYS_SUCCESS.clone());
    context
        .build_script(&out_point, args)
        .expect("always-success script")
}

pub(super) fn always_success_lock(context: &mut Context) -> Script {
    always_success_script(context, Bytes::new())
}

pub(super) fn named_always_success_lock(context: &mut Context, name: &[u8]) -> Script {
    always_success_script(context, Bytes::from(name.to_vec()))
}

pub(super) fn data1_script(context: &mut Context, binary: &str, args: Bytes) -> Script {
    let out_point = deployed_cell(context, load_binary(binary));
    context
        .build_script_with_hash_type(&out_point, ScriptHashType::Data1, args)
        .expect("data1 script")
}

pub(super) fn limit_order_script(context: &mut Context) -> Script {
    data1_script(context, "limit_order", Bytes::new())
}

pub(super) fn ickb_logic_script(context: &mut Context) -> Script {
    data1_script(context, "ickb_logic", Bytes::new())
}

pub(super) fn xudt_script(context: &mut Context, owner_script: &Script) -> Script {
    data1_script(context, "xudt", xudt_args(owner_script))
}

pub(super) fn helper_type_script(context: &mut Context) -> Script {
    named_always_success_lock(context, b"helper-type")
}

pub(super) fn named_lock_and_helper_type_scripts(context: &mut Context, name: &[u8]) -> (Script, Script) {
    let lock = named_always_success_lock(context, name);
    let helper_type = helper_type_script(context);
    // Tuple order is `(lock, helper_type)` so callers can drop it straight into output builders.
    (lock, helper_type)
}

pub(super) fn limit_order_and_helper_type_scripts(context: &mut Context) -> (Script, Script) {
    let limit_order = limit_order_script(context);
    let helper_type = helper_type_script(context);
    // Returns the spend lock first; callers still choose the order data and capacity.
    (limit_order, helper_type)
}

pub(super) fn funding_limit_order_and_helper_type_scripts(context: &mut Context) -> (Script, Script, Script) {
    let funding_lock = always_success_lock(context);
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(context);
    // Return order matches the common minting call sites: funding lock first, then the order pair.
    (funding_lock, limit_order, helper_type)
}

pub(super) fn ickb_logic_and_limit_order_scripts(context: &mut Context) -> (Script, Script) {
    let ickb_logic = ickb_logic_script(context);
    let limit_order = limit_order_script(context);
    (ickb_logic, limit_order)
}

pub(super) fn ickb_logic_and_dao_scripts(context: &mut Context) -> (Script, Script) {
    let ickb_logic = ickb_logic_script(context);
    let dao = dao_script(context);
    (ickb_logic, dao)
}

pub(super) fn ickb_logic_and_xudt_scripts(context: &mut Context) -> (Script, Script) {
    let ickb_logic = ickb_logic_script(context);
    let xudt = xudt_script(context, &ickb_logic);
    (ickb_logic, xudt)
}

pub(super) fn ickb_logic_dao_and_xudt_scripts(context: &mut Context) -> (Script, Script, Script) {
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(context);
    let xudt = xudt_script(context, &ickb_logic);
    (ickb_logic, dao, xudt)
}

pub(super) fn ickb_logic_owned_owner_dao_and_xudt_scripts(
    context: &mut Context,
) -> (Script, Script, Script, Script) {
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(context);
    let owned_owner = owned_owner_script(context);
    let xudt = xudt_script(context, &ickb_logic);
    // Keep the tuple grouped as: primary lock, owner script, DAO type, then the derived xUDT type.
    (ickb_logic, owned_owner, dao, xudt)
}

pub(super) fn ickb_logic_limit_order_and_xudt_scripts(context: &mut Context) -> (Script, Script, Script) {
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(context);
    let xudt = xudt_script(context, &ickb_logic);
    (ickb_logic, limit_order, xudt)
}

pub(super) fn ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(
    context: &mut Context,
) -> (Script, Script, Script, Script, Script) {
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(context);
    let owned_owner = owned_owner_script(context);
    let dao = dao_script(context);
    let xudt = xudt_script(context, &ickb_logic);
    // The tuple stays in dependency order: main lock, order lock, auxiliary owner, DAO, then xUDT.
    (ickb_logic, limit_order, owned_owner, dao, xudt)
}

pub(super) fn dao_dep_out_point(context: &mut Context) -> OutPoint {
    let data = Bytes::from(BUNDLED_CELL.get("specs/cells/dao").expect("bundled dao cell").into_owned());
    let lock = always_success_lock(context);
    let type_script = build_genesis_type_id_script(OUTPUT_INDEX_DAO);
    let cell = CellOutput::new_builder()
        .lock(lock)
        .type_(Some(type_script).pack())
        .capacity(capacity_for_data(data.len() as u64).pack())
        .build();
    context.create_cell(cell, data)
}

pub(super) fn dao_script(context: &mut Context) -> Script {
    let dao_out_point = dao_dep_out_point(context);
    context
        .build_script(&dao_out_point, Bytes::new())
        .expect("dao script")
}

pub(super) fn owned_owner_script(context: &mut Context) -> Script {
    data1_script(context, "owned_owner", Bytes::new())
}

// Capacity and DAO accounting helpers.
pub(super) fn capacity_for_data(data_len: u64) -> u64 {
    100 * SHANNONS + data_len
}

pub(super) fn deposit_capacity(lock: &Script, type_: &Script, data_len: usize, unused_capacity: u64) -> u64 {
    let output = CellOutput::new_builder()
        .lock(lock.clone())
        .type_(Some(type_.clone()).pack())
        .build();
    let occupied = output
        .occupied_capacity(
            ckb_testtool::ckb_types::core::Capacity::bytes(data_len)
                .expect("occupied capacity bytes"),
        )
        .expect("occupied capacity")
        .as_u64();
    occupied + unused_capacity
}

pub(super) fn deposit_total_capacity_and_header(
    lock: &Script,
    dao: &Script,
    deposit_amount: u64,
    deposit_number: u64,
) -> (u64, ckb_testtool::ckb_types::core::HeaderView) {
    // Returns `(total_capacity, deposit_header)`; callers must still create and link the DAO cell.
    (
        deposit_capacity(lock, dao, 8, deposit_amount),
        gen_header(deposit_number, GENESIS_AR as u64, 35, 1000, 1000),
    )
}

pub(super) fn dao_maximum_withdraw_capacity(
    output: &CellOutput,
    output_data_len: usize,
    deposit_ar: u64,
    withdraw_ar: u64,
) -> u64 {
    let occupied = output
        .occupied_capacity(
            ckb_testtool::ckb_types::core::Capacity::bytes(output_data_len)
                .expect("occupied capacity bytes"),
        )
        .expect("occupied capacity")
        .as_u64();
    let total: u64 = output.capacity().unpack();
    let counted = total - occupied;
    (u128::from(counted) * u128::from(withdraw_ar) / u128::from(deposit_ar)) as u64 + occupied
}

// Header and transaction-shape helpers.
pub(super) fn insert_header_for_cell(context: &mut Context, out_point: &OutPoint, number: u64, ar: u64) -> Byte32 {
    let header = HeaderBuilder::default()
        .number(number.pack())
        .dao(pack_dao_data(
            ar,
            ckb_testtool::ckb_types::core::Capacity::zero(),
            ckb_testtool::ckb_types::core::Capacity::zero(),
            ckb_testtool::ckb_types::core::Capacity::zero(),
        ))
        .build();
    let hash = header.hash();
    context.insert_header(header);
    context.link_cell_with_block(out_point.clone(), hash.clone(), 0);
    hash
}

pub(super) fn gen_header(
    number: u64,
    ar: u64,
    epoch_number: u64,
    epoch_start_block_number: u64,
    epoch_length: u64,
) -> ckb_testtool::ckb_types::core::HeaderView {
    let epoch_ext = EpochExt::new_builder()
        .number(epoch_number)
        .start_number(epoch_start_block_number)
        .length(epoch_length)
        .build();
    HeaderBuilder::default()
        .number(number.pack())
        .epoch(epoch_ext.number_with_fraction(number).pack())
        .dao(pack_dao_data(
            ar,
            ckb_testtool::ckb_types::core::Capacity::zero(),
            ckb_testtool::ckb_types::core::Capacity::zero(),
            ckb_testtool::ckb_types::core::Capacity::zero(),
        ))
        .build()
}

pub(super) fn link_cell_to_header(
    context: &mut Context,
    out_point: &OutPoint,
    header: &ckb_testtool::ckb_types::core::HeaderView,
) {
    context.insert_header(header.clone());
    context.link_cell_with_block(out_point.clone(), header.hash(), 0);
}

pub(super) fn seed_verified_output(
    context: &mut Context,
    tx: &TransactionView,
    index: usize,
    data: Bytes,
) -> OutPoint {
    let out_point = OutPoint::new(tx.hash(), index as u32);
    // Seed the verified live cell back into the mock chain at the same out point so replay-style
    // follow-up transactions keep the original tx hash and output index.
    context.create_cell_with_out_point(
        out_point.clone(),
        tx.outputs().get(index).expect("verified output"),
        data,
    );
    out_point
}

pub(super) fn create_receipt_input(
    context: &mut Context,
    lock: Script,
    ickb_logic: &Script,
    data: Bytes,
    block_number: u64,
    accumulated_rate: u64,
) -> (OutPoint, Byte32) {
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(data.len() as u64).pack())
            .lock(lock)
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        data,
    );
    let receipt_header = insert_header_for_cell(context, &receipt_out_point, block_number, accumulated_rate);
    // Returns `(receipt_out_point, receipt_header_hash)` after linking the new cell to that header.
    (receipt_out_point, receipt_header)
}

pub(super) fn create_withdrawal_inputs(
    context: &mut Context,
    ickb_logic: &Script,
    dao: &Script,
    xudt: &Script,
    owner_lock: Script,
    deposit_amount: u64,
    deposit_number: u64,
) -> (u64, ckb_testtool::ckb_types::core::HeaderView, OutPoint, OutPoint) {
    let (deposit_total_capacity, deposit_header) =
        deposit_total_capacity_and_header(ickb_logic, dao, deposit_amount, deposit_number);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(context, &deposit_input, &deposit_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock)
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );
    // Returns `(deposit_total_capacity, deposit_header, deposit_input, udt_input)`; callers still supply the matching withdraw header deps.
    (deposit_total_capacity, deposit_header, deposit_input, udt_input)
}

pub(super) fn build_real_limit_order_and_master(
    context: &mut Context,
    owner_lock: Script,
    helper_type: Script,
) -> (OutPoint, OutPoint) {
    build_real_limit_order_and_master_with_capacity(context, owner_lock, helper_type, 1_500 * SHANNONS)
}

pub(super) fn build_real_limit_order_and_master_with_capacity(
    context: &mut Context,
    owner_lock: Script,
    helper_type: Script,
    capacity: u64,
) -> (OutPoint, OutPoint) {
    let funding_lock = always_success_lock(context);
    let limit_order = limit_order_script(context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((capacity + 500u64).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity.pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(limit_order).pack())
                .build(),
        )
        .outputs_data(vec![order_data_mint(0, 1, (1, 1)), Bytes::new()].pack())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("real limit order mint should verify");
    let order_out_point = seed_verified_output(context, &tx, 0, order_data_mint(0, 1, (1, 1)));
    let master_out_point = seed_verified_output(context, &tx, 1, Bytes::new());
    // Returns `(order_out_point, master_out_point)` seeded from the verified tx; callers still add deps when spending them later.
    (order_out_point, master_out_point)
}

pub(super) fn assert_lock_only_limit_order_spend_error(data: Bytes, expected_error: i8) {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (limit_order, output_type) = limit_order_and_helper_type_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let forged_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(limit_order.clone())
        .type_(Some(output_type).pack())
        .build();
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(forged_output.clone())
        .output_data(data.clone().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("lock-only forged order should be creatable");

    let forged_out_point = context.create_cell(forged_output, data);
    let spend_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(forged_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, expected_error);
}
