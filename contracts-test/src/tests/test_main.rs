use super::{types_utils::merkle_root, MAX_CYCLES};
use super::{DUMMY_LOCK_BIN, MAIN_CONTRACT_BIN};
use ckb_contract_tool::{
    ckb_hash::{blake2b_256, new_blake2b},
    Context, TxBuilder,
};
use ckb_merkle_mountain_range::{leaf_index_to_pos, util::MemMMR, Merge};
use godwoken_types::bytes::Bytes;
use godwoken_types::packed::{
    Account, AccountScript, Action, AgBlock, Deposit, GlobalState, Register, SubmitBlock, Tx,
    TxVec, WitnessArgs,
};
use godwoken_types::prelude::*;
use rand::{thread_rng, Rng};

const AGGREGATOR_REQUIRED_BALANCE: u64 = 2000;
const NEW_ACCOUNT_REQUIRED_BALANCE: u64 = 1000;

struct HashMerge;

impl Merge for HashMerge {
    type Item = [u8; 32];
    fn merge(left: &Self::Item, right: &Self::Item) -> Self::Item {
        let mut merge_result = [0u8; 32];
        let mut hasher = new_blake2b();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize(&mut merge_result);
        merge_result
    }
}

type HashMMR = MemMMR<[u8; 32], HashMerge>;

#[derive(Default)]
struct GlobalStateContext {
    account_entries: Vec<Account>,
    block_root: [u8; 32],
    block_mmr: HashMMR,
}

impl GlobalStateContext {
    fn new() -> Self {
        Default::default()
    }

    fn get_global_state(&self) -> GlobalState {
        GlobalState::new_builder()
            .account_root(self.account_root().pack())
            .block_root(self.block_root.pack())
            .build()
    }

    fn account_root(&self) -> [u8; 32] {
        let mut account_root = [0u8; 32];
        if self.account_entries.is_empty() {
            return account_root;
        }
        let account_mmr_root = self.build_account_mmr().get_root().expect("mmr root");
        let entries_count: u32 = self.account_entries.len() as u32;
        let mut hasher = new_blake2b();
        hasher.update(&entries_count.to_le_bytes());
        hasher.update(&account_mmr_root);
        hasher.finalize(&mut account_root);
        account_root
    }

    fn build_account_mmr(&self) -> HashMMR {
        let mut mmr = HashMMR::default();
        for account in &self.account_entries {
            let account_hash = blake2b_256(account.as_slice());
            mmr.push(account_hash).expect("mmr push");
        }
        mmr
    }

    fn push_account(&mut self, account: Account) {
        let index = Unpack::<u32>::unpack(&account.index()) as usize;
        if index == self.account_entries.len() {
            self.account_entries.push(account);
        } else {
            self.account_entries[index] = account;
        }
    }

    fn gen_account_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .build_account_mmr()
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    fn submit_block(&mut self, block: AgBlock, count: u32) {
        let block_hash = blake2b_256(block.as_slice());
        self.block_mmr.push(block_hash).expect("mmr push");
        let block_mmr_root = self.block_mmr.get_root().expect("mmr root");
        let mut hasher = new_blake2b();
        hasher.update(&count.to_le_bytes());
        hasher.update(&block_mmr_root);
        hasher.finalize(&mut self.block_root);
    }

    fn gen_block_merkle_proof(&self, leaf_index: u32) -> (u64, Vec<[u8; 32]>) {
        let proof = self
            .block_mmr
            .gen_proof(leaf_index_to_pos(leaf_index.into()))
            .expect("result");
        (proof.mmr_size(), proof.proof_items().to_owned())
    }

    fn apply_tx(&mut self, tx: &Tx, fee_to: u32) {
        let tx_fee: u64 = {
            let tx_fee: u32 = tx.fee().unpack();
            tx_fee.into()
        };
        let args: Bytes = tx.args().unpack();
        let to_index: u64 = {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&args[..4]);
            u32::from_le_bytes(buf).into()
        };
        let amount: u64 = {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&args[4..]);
            u32::from_le_bytes(buf).into()
        };
        let from_account =
            &self.account_entries[Unpack::<u32>::unpack(&tx.account_index()) as usize];
        let from_account = from_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = from_account.balance().unpack();
                (balance - amount - tx_fee).pack()
            })
            .nonce({
                let nonce: u32 = from_account.nonce().unpack();
                (nonce + 1).pack()
            })
            .build();
        let to_account = &self.account_entries[to_index as usize];
        let to_account = to_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = to_account.balance().unpack();
                (balance + amount).pack()
            })
            .build();
        let fee_account = &self.account_entries[fee_to as usize];
        let fee_account = fee_account
            .clone()
            .as_builder()
            .balance({
                let balance: u64 = fee_account.balance().unpack();
                (balance + tx_fee).pack()
            })
            .build();
        self.push_account(from_account);
        self.push_account(to_account);
        self.push_account(fee_account);
    }
}

#[test]
fn test_account_register() {
    let mut context = GlobalStateContext::new();
    let global_state = context.get_global_state();
    // insert few entries
    let mut last_account: Option<Account> = None;
    let mut global_state = global_state;
    let mut original_amount = 0;
    for index in 0u32..=5u32 {
        let is_aggregator = index < 2;
        let deposit_amount = if is_aggregator {
            AGGREGATOR_REQUIRED_BALANCE
        } else {
            NEW_ACCOUNT_REQUIRED_BALANCE
        };
        original_amount += deposit_amount;
        let account = {
            let mut pubkey = [0u8; 20];
            let mut rng = thread_rng();
            rng.fill(&mut pubkey);
            Account::new_builder()
                .index(index.pack())
                .script(
                    AccountScript::new_builder()
                        .args(Bytes::from(pubkey.to_vec()).pack())
                        .build(),
                )
                .is_ag({
                    if is_aggregator {
                        1.into()
                    } else {
                        0.into()
                    }
                })
                .balance(deposit_amount.pack())
                .build()
        };
        let register = match last_account {
            None => {
                // first account
                Register::new_builder().account(account.clone()).build()
            }
            Some(last_account) => {
                let (_, proof) = context.gen_account_merkle_proof(last_account.index().unpack());
                Register::new_builder()
                    .account(account.clone())
                    .last_account_hash(blake2b_256(last_account.as_slice()).pack())
                    .proof(
                        proof
                            .into_iter()
                            .map(|i| i.pack())
                            .collect::<Vec<_>>()
                            .pack(),
                    )
                    .build()
            }
        };
        let action = Action::new_builder().set(register).build();
        context.push_account(account.clone());
        let new_global_state = context.get_global_state();
        let witness = WitnessArgs::new_builder()
            .output_type(Some(action.as_bytes()).pack())
            .build();
        let contract_bin = MAIN_CONTRACT_BIN.to_owned();
        let mut context = Context::default();
        context.deploy_contract(DUMMY_LOCK_BIN.to_owned());
        context.deploy_contract(contract_bin.clone());
        let tx = TxBuilder::default()
            .lock_bin(DUMMY_LOCK_BIN.to_owned())
            .type_bin(contract_bin)
            .previous_output_data(global_state.as_slice().into())
            .input_capacity(original_amount)
            .output_capacity(original_amount + deposit_amount)
            .witnesses(vec![witness.as_slice().into()])
            .outputs_data(vec![new_global_state.as_slice().into()])
            .inject_and_build(&mut context)
            .expect("build tx");
        let verify_result = context.verify_tx(&tx, MAX_CYCLES);
        verify_result.expect("pass verification");
        last_account = Some(account);
        global_state = new_global_state;
    }
}

#[test]
fn test_deposit() {
    let mut context = GlobalStateContext::new();
    // prepare a account account
    let account = Account::new_builder().build();
    context.push_account(account.clone());
    let global_state = context.get_global_state();

    let original_amount = 12u64;
    let deposit_amount = 42u64;

    // deposit money
    let new_account = {
        let balance: u64 = account.balance().unpack();
        account
            .clone()
            .as_builder()
            .balance((balance + deposit_amount).pack())
            .build()
    };
    let (_, proof) = context.gen_account_merkle_proof(account.index().unpack());
    let deposit = Deposit::new_builder()
        .old_account(account)
        .new_account(new_account.clone())
        .count(1u32.pack())
        .proof(
            proof
                .into_iter()
                .map(|i| i.pack())
                .collect::<Vec<_>>()
                .pack(),
        )
        .build();
    let action = Action::new_builder().set(deposit).build();
    let new_global_state = {
        let mut new_context = GlobalStateContext::new();
        new_context.push_account(new_account);
        new_context.get_global_state()
    };

    // update tx witness
    let witness = WitnessArgs::new_builder()
        .output_type(Some(action.as_bytes()).pack())
        .build();
    let contract_bin = MAIN_CONTRACT_BIN.to_owned();
    let mut context = Context::default();
    context.deploy_contract(DUMMY_LOCK_BIN.to_owned());
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_bin(DUMMY_LOCK_BIN.to_owned())
        .type_bin(contract_bin)
        .previous_output_data(global_state.as_slice().into())
        .input_capacity(original_amount)
        .output_capacity(original_amount + deposit_amount)
        .witnesses(vec![witness.as_slice().into()])
        .outputs_data(vec![new_global_state.as_slice().into()])
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    verify_result.expect("pass verification");
}

#[test]
fn test_submit_block() {
    let mut context = GlobalStateContext::new();

    // prepare account entries
    let account_a = Account::new_builder()
        .balance(20u64.pack())
        .index(0u32.pack())
        .build();
    let account_b = Account::new_builder()
        .balance(100u64.pack())
        .index(1u32.pack())
        .build();
    let (privkey, pubkey) = {
        let mut rng = thread_rng();
        let privkey = secp256k1::SecretKey::random(&mut rng);
        let pubkey = secp256k1::PublicKey::from_secret_key(&privkey);
        (privkey, pubkey)
    };
    let pubkey_hash = {
        let pubkey_bytes = pubkey.serialize_compressed();
        let mut hasher = new_blake2b();
        hasher.update(&pubkey_bytes);
        let mut hash = [0u8; 20];
        hasher.finalize(&mut hash);
        hash
    };
    let account_ag = Account::new_builder()
        .balance(AGGREGATOR_REQUIRED_BALANCE.pack())
        .index(2u32.pack())
        .script(
            AccountScript::new_builder()
                .args(Bytes::from(pubkey_hash.to_vec()).pack())
                .build(),
        )
        .is_ag(1u8.into())
        .build();
    context.push_account(account_a.clone());
    context.push_account(account_b.clone());
    context.push_account(account_ag.clone());
    // aggregator proof
    let (_account_mmr_size, account_proof) =
        context.gen_account_merkle_proof(account_ag.index().unpack());

    let global_state = context.get_global_state();
    let previous_account_root = global_state.account_root().unpack();

    let transfer_tx = Tx::new_builder()
        .account_index(account_a.index())
        .fee(3u32.pack())
        .nonce(1u32.pack())
        .args({
            let mut args = vec![0u8; 8];
            let to_index: u32 = account_b.index().unpack();
            args[..4].copy_from_slice(&to_index.to_le_bytes());
            args[4..].copy_from_slice(&15u32.to_le_bytes());
            args.pack()
        })
        .build();

    context.apply_tx(&transfer_tx, account_ag.index().unpack());

    // new account root
    let new_account_root = context.account_root();

    let original_amount = 120u64;
    // send money
    let tx_root = merkle_root(&[blake2b_256(transfer_tx.as_slice()).pack()]);

    let block = AgBlock::new_builder()
        .number(0u32.pack())
        .tx_root(tx_root)
        .ag_index(account_ag.index())
        .previous_account_root(previous_account_root.pack())
        .current_account_root(new_account_root.pack())
        .build();
    let ag_sig = {
        let mut hasher = new_blake2b();
        hasher.update(block.as_slice());
        let mut block_hash = [0u8; 32];
        hasher.finalize(&mut block_hash);
        let msg = secp256k1::Message::parse(&block_hash);
        let (signature, rec_id) = secp256k1::sign(&msg, &privkey);
        let mut sig = [0u8; 65];
        sig[..64].copy_from_slice(&signature.serialize());
        sig[64] = rec_id.serialize();
        sig
    };
    let block = block.as_builder().ag_sig(ag_sig.pack()).build();

    let (_block_mmr_size, block_proof) = context.gen_block_merkle_proof(0);
    let submit_block = {
        let txs = TxVec::new_builder().set(vec![transfer_tx]).build();
        SubmitBlock::new_builder()
            .txs(txs)
            .block(block.clone())
            .block_proof(
                block_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .ag_account(account_ag)
            .account_proof(
                account_proof
                    .into_iter()
                    .map(|i| i.pack())
                    .collect::<Vec<_>>()
                    .pack(),
            )
            .account_count(3u32.pack())
            .build()
    };
    let action = Action::new_builder().set(submit_block).build();

    // submit block
    context.submit_block(block, 1);
    let new_global_state = context.get_global_state();

    // update tx witness
    let witness = WitnessArgs::new_builder()
        .output_type(Some(action.as_bytes()).pack())
        .build();
    let contract_bin = MAIN_CONTRACT_BIN.to_owned();
    let mut context = Context::default();
    context.deploy_contract(DUMMY_LOCK_BIN.to_owned());
    context.deploy_contract(contract_bin.clone());
    let tx = TxBuilder::default()
        .lock_bin(DUMMY_LOCK_BIN.to_owned())
        .type_bin(contract_bin)
        .previous_output_data(global_state.as_slice().into())
        .input_capacity(original_amount)
        .output_capacity(original_amount)
        .witnesses(vec![witness.as_slice().into()])
        .outputs_data(vec![new_global_state.as_slice().into()])
        .inject_and_build(&mut context)
        .expect("build tx");
    let verify_result = context.verify_tx(&tx, MAX_CYCLES);
    verify_result.expect("pass verification");
}
