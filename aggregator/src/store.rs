/// Storage for contract related data

pub struct Store {
    accounts: Vec<(Account, SMT)>,
    account_count: u32,
    blocks: HashMMR,
    block_count: u32,
};
