use super::*;
use ckb_testtool::ckb_types::core::HeaderView;

// The one way tests build transactions: cells and inputs as plain values, assembled by `TxSpec`,
// so every test, generator and search shares the same construction and the same verification.

/// The scripts every case can use, deployed once per case context.
pub(super) struct Env {
    pub(super) user: Script,
    pub(super) funding: Script,
    pub(super) ickb: Script,
    pub(super) dao: Script,
    pub(super) xudt: Script,
    pub(super) owned_owner: Script,
    pub(super) limit_order: Script,
    pub(super) helper: Script,
    // Same code with non-empty args: different script hashes, so different groups.
    pub(super) siblings: Vec<Script>,
}

impl Env {
    pub(super) fn new(context: &mut Context) -> Self {
        let user = named_always_success_lock(context, b"fuzz-user");
        let funding = named_always_success_lock(context, b"funding");
        let (ickb, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(context);
        let limit_order = limit_order_script(context);
        let helper = helper_type_script(context);
        let siblings = vec![
            data1_script(context, "ickb_logic", Bytes::from(vec![1])),
            data1_script(context, "owned_owner", Bytes::from(vec![1])),
            data1_script(context, "limit_order", Bytes::from(vec![1])),
        ];
        let sibling_ickb = siblings[0].clone();
        let mut siblings = siblings;
        siblings.push(xudt_script(context, &sibling_ickb));
        Env { user, funding, ickb, dao, xudt, owned_owner, limit_order, helper, siblings }
    }

    pub(super) fn script_pool(&self) -> Vec<Script> {
        let mut pool = vec![
            self.user.clone(),
            self.ickb.clone(),
            self.dao.clone(),
            self.xudt.clone(),
            self.owned_owner.clone(),
            self.limit_order.clone(),
            self.helper.clone(),
        ];
        pool.extend(self.siblings.iter().cloned());
        pool
    }
}

#[derive(Clone, Debug)]
pub(super) struct Cell {
    pub(super) capacity: u64,
    pub(super) lock: Script,
    pub(super) type_: Option<Script>,
    pub(super) data: Vec<u8>,
}

/// A cell under `lock` with nothing else.
pub(super) fn plain(lock: &Script, capacity: u64) -> Cell {
    Cell { capacity, lock: lock.clone(), type_: None, data: vec![] }
}

impl Cell {
    pub(super) fn output(&self) -> CellOutput {
        cell(self.capacity, &self.lock, self.type_.as_ref())
    }

    pub(super) fn occupied(&self) -> u64 {
        self.output()
            .occupied_capacity(ckb_testtool::ckb_types::core::Capacity::bytes(self.data.len()).expect("data length"))
            .expect("occupied capacity")
            .as_u64()
    }
}

#[derive(Clone, Debug)]
pub(super) struct In {
    pub(super) cell: Cell,
    pub(super) out_point: OutPoint,
    pub(super) header: Option<HeaderView>,
    pub(super) since: u64,
    pub(super) witness: Bytes,
}

#[derive(Clone, Debug)]
pub(super) struct Out {
    pub(super) cell: Cell,
    pub(super) change: bool,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TxSpec {
    pub(super) inputs: Vec<In>,
    pub(super) outputs: Vec<Out>,
    pub(super) header_deps: Vec<HeaderView>,
}

impl TxSpec {
    /// Gives the change output whatever capacity the other outputs leave, as a wallet would.
    /// Returns false when no realistic change exists.
    pub(super) fn balance(&mut self) -> bool {
        let Some(change) = self.outputs.iter().position(|o| o.change) else {
            return true;
        };
        let inputs: u128 = self.inputs.iter().map(|i| u128::from(i.cell.capacity)).sum();
        let others: u128 = self
            .outputs
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != change)
            .map(|(_, o)| u128::from(o.cell.capacity))
            .sum();
        let Some(rest) = inputs.checked_sub(others) else {
            return false;
        };
        let Ok(rest) = u64::try_from(rest) else {
            return false;
        };
        self.outputs[change].cell.capacity = rest;
        rest >= self.outputs[change].cell.occupied()
    }

    pub(super) fn build(&self, context: &mut Context) -> TransactionView {
        let mut builder = TransactionBuilder::default();
        // Every live cell sits in some block; cells whose header nobody needs get one outside the
        // header deps, so a script asking for it sees ItemMissing as on chain.
        let unlisted = gen_header(1_999, GENESIS_AR, 35, 1000, 1000);
        for i in &self.inputs {
            context.create_cell_with_out_point(i.out_point.clone(), i.cell.output(), Bytes::from(i.cell.data.clone()));
            let h = i.header.as_ref().unwrap_or(&unlisted);
            context.insert_header(h.clone());
            context.link_cell_with_block(i.out_point.clone(), h.hash(), 0);
            builder = builder
                .input(CellInput::new_builder().previous_output(i.out_point.clone()).since(i.since).build())
                .witness(i.witness.pack());
        }
        for o in &self.outputs {
            builder = builder.output(o.cell.output()).output_data(Bytes::from(o.cell.data.clone()).pack());
        }
        for h in &self.header_deps {
            context.insert_header(h.clone());
            builder = builder.header_dep(h.hash());
        }
        context.complete_tx(builder.build())
    }
}

impl Env {
    /// A cell under the test user's lock with nothing else.
    pub(super) fn plain(&self, capacity: u64) -> Cell {
        plain(&self.user, capacity)
    }

    /// The plain 1,000 CKB input a wallet adds to fund a transaction.
    pub(super) fn funding_input(&self) -> In {
        In::new(plain(&self.funding, 1_000 * CKB))
    }

    /// An iCKB xUDT cell holding `amount`, at its occupied capacity.
    pub(super) fn udt(&self, amount: u128) -> Cell {
        Cell { capacity: occupied_capacity(&self.user, &self.xudt, 16), lock: self.user.clone(), type_: Some(self.xudt.clone()), data: udt_data(amount).to_vec() }
    }

    /// A NervosDAO deposit under `lock` with `amount` of unused capacity.
    pub(super) fn deposit(&self, lock: &Script, amount: u64) -> Cell {
        Cell { capacity: deposit_capacity(lock, &self.dao, 8, amount), lock: lock.clone(), type_: Some(self.dao.clone()), data: vec![0; 8] }
    }

    /// A receipt for `quantity` deposits of `amount` each.
    pub(super) fn receipt(&self, quantity: u32, amount: u64) -> Cell {
        Cell { capacity: occupied_capacity(&self.user, &self.ickb, 12), lock: self.user.clone(), type_: Some(self.ickb.clone()), data: receipt_data(quantity, amount).to_vec() }
    }

    /// An owner cell pointing `distance` cells away to its owned withdrawal request.
    pub(super) fn owner(&self, distance: i32) -> Cell {
        Cell { capacity: occupied_capacity(&self.user, &self.owned_owner, 4), lock: self.user.clone(), type_: Some(self.owned_owner.clone()), data: distance.to_le_bytes().to_vec() }
    }

    /// A limit order master cell.
    pub(super) fn master(&self) -> Cell {
        Cell { capacity: occupied_capacity(&self.user, &self.limit_order, 0), lock: self.user.clone(), type_: Some(self.limit_order.clone()), data: vec![] }
    }
}

impl Cell {
    /// The same cell holding exactly its occupied capacity.
    pub(super) fn sized(self) -> Cell {
        Cell { capacity: self.occupied(), ..self }
    }

    /// The same cell under `lock`, re-sized to its occupied capacity, since that depends on the lock.
    pub(super) fn under(self, lock: &Script) -> Cell {
        Cell { lock: lock.clone(), ..self }.sized()
    }
}

impl From<Cell> for Out {
    fn from(cell: Cell) -> Self {
        Out { cell, change: false }
    }
}

impl In {
    /// The cell as a live input at an out point no other input uses, with no header and since 0.
    pub(super) fn new(cell: Cell) -> Self {
        // Per thread, and libtest runs each test on its own thread, so a test's out points never
        // depend on which other tests run beside it.
        thread_local!(static NEXT: std::cell::Cell<u64> = const { std::cell::Cell::new(1) });
        let next = NEXT.with(|n| n.replace(n.get() + 1));
        let mut tx_hash = [0xa5; 32];
        tx_hash[..8].copy_from_slice(&next.to_le_bytes());
        In { cell, out_point: OutPoint::new(tx_hash.pack(), 0), header: None, since: 0, witness: Bytes::new() }
    }

    /// The same input, created in `header`'s block.
    pub(super) fn in_block(self, header: &HeaderView) -> In {
        In { header: Some(header.clone()), ..self }
    }
}

impl TxSpec {
    pub(super) fn verify(&self, context: &mut Context) -> Result<u64, Error> {
        let tx = self.build(context);
        context.verify(&tx, MAX_CYCLES)
    }

    /// Verifies the transaction and returns its outputs as live inputs at their out points, so the
    /// next step of a lineage can spend them.
    pub(super) fn accept(&self, context: &mut Context, expect: &str) -> Vec<In> {
        let tx = self.build(context);
        context.verify(&tx, MAX_CYCLES).expect(expect);
        (0u32..).zip(&self.outputs).map(|(i, o)| In { out_point: OutPoint::new(tx.hash(), i), ..In::new(o.cell.clone()) }).collect()
    }

    /// Asserts the scripts reject the transaction with `err_code`.
    pub(super) fn fail(&self, context: &mut Context, err_code: i8) {
        assert_script_error(self.verify(context).expect_err("the transaction should be rejected"), err_code);
    }
}
