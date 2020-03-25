use crate::constants::{
    CHALLENGE_CELL_WAIT_BLOCKS, CHALLENGE_CONTRACT_CODE_HASH, CHALLENGE_REWARD_RATE, CKB_TOKEN_ID,
    HASH_SIZE,
};
use crate::error::Error;
use alloc::vec;
use alloc::vec::Vec;
use ckb_std::{ckb_constants::*, since, syscalls};
use godwoken_types::{cache::KVMap, core::Index, packed::*, prelude::*};
use godwoken_utils::{
    hash::new_blake2b,
    mmr::compute_block_root,
    smt::{self, compute_root_with_proof, Value, ValueTrait},
};

pub struct RevertBlockVerifier<'a> {
    action: RevertBlockReader<'a>,
    old_state: GlobalStateReader<'a>,
    new_state: GlobalStateReader<'a>,
}

impl<'a> RevertBlockVerifier<'a> {
    pub fn new(
        old_state: GlobalStateReader<'a>,
        new_state: GlobalStateReader<'a>,
        revert_block: RevertBlockReader<'a>,
    ) -> RevertBlockVerifier<'a> {
        RevertBlockVerifier {
            action: revert_block,
            old_state,
            new_state,
        }
    }

    fn verify_challenge_cell(&self, challenge_cell_index: usize) -> Result<(), Error> {
        const SINCE_LEN: usize = 8;

        let buf = syscalls::load_cell_by_field(
            HASH_SIZE,
            0,
            challenge_cell_index,
            Source::Input,
            CellField::Type,
        )
        .expect("load challenge cell");
        let challenge_cell_type = match ScriptReader::verify(&buf, false) {
            Ok(()) => Script::new_unchecked(buf.into()),
            Err(_err) => return Err(Error::InvalidScript),
        };
        // verify challenge cell's type
        if challenge_cell_type.code_hash().unpack() != CHALLENGE_CONTRACT_CODE_HASH {
            return Err(Error::InvalidChallengeCell);
        }
        // verify challenge args
        let args: Vec<_> = challenge_cell_type.args().unpack();
        let challenge_args = match ChallengeArgsReader::verify(&args, false) {
            Ok(()) => ChallengeArgs::new_unchecked(args.into()),
            Err(_err) => return Err(Error::InvalidChallengeCell),
        };
        let script_hash = syscalls::load_script_hash(HASH_SIZE, 0).expect("load script");
        if challenge_args.main_type_hash().unpack()[..] != script_hash[..] {
            return Err(Error::InvalidChallengeCell);
        }
        // verify challenge cell wait time
        let buf = syscalls::load_input_by_field(
            SINCE_LEN,
            0,
            challenge_cell_index,
            Source::GroupInput,
            InputField::Since,
        )
        .map_err(|_| Error::InvalidSince)?;
        let input_since = {
            let mut raw_since = [0u8; 8];
            raw_since.copy_from_slice(&buf);
            since::Since::new(u64::from_le_bytes(raw_since))
        };
        if !input_since.is_relative() {
            return Err(Error::InvalidSince);
        }
        let wait_blocks = input_since
            .extract_lock_value()
            .and_then(|value| value.block_number())
            .ok_or(Error::InvalidSince)?;
        if wait_blocks < CHALLENGE_CELL_WAIT_BLOCKS {
            return Err(Error::InvalidSince);
        }
        Ok(())
    }

    fn verify_block(
        &self,
        block: AgBlockReader<'a>,
        block_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        if block.is_reverted_block() {
            // A penalized block can't be invalid since it is generated on-chain
            return Err(Error::TryRevertRevertedBlock);
        }
        let block_number: u64 = block.number().unpack();
        let block_hash = {
            let mut hasher = new_blake2b();
            hasher.update(block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let block_root = compute_block_root(
            vec![(block_number as usize, block_hash)],
            block_number + 1,
            block_proof,
        )
        .map_err(|_| Error::InvalidBlockMerkleProof)?;
        if &block_root != self.old_state.block_root().raw_data() {
            return Err(Error::InvalidBlockMerkleProof);
        }
        Ok(())
    }

    fn verify_accounts(
        &self,
        ag_account: AccountReader<'a>,
        ag_kv: KVMap,
        chal_account: AccountReader<'a>,
        chal_kv: KVMap,
        block: AgBlockReader<'a>,
        leaves_path: Vec<Vec<u8>>,
        merkle_branches: Vec<(smt::H256, u8)>,
    ) -> Result<(), Error> {
        let leaves = accounts_to_merkle_leaves(&[(ag_account, ag_kv), (chal_account, chal_kv)]);
        let calculated_root: [u8; 32] =
            compute_root_with_proof(leaves, leaves_path, merkle_branches)
                .map_err(|_| Error::InvalidAccountMerkleProof)?
                .into();
        if &calculated_root != block.prev_account_root().raw_data() {
            return Err(Error::InvalidAccountMerkleProof);
        }
        Ok(())
    }

    pub fn verify_reverted_state(
        &self,
        reverted_account_root: [u8; 32],
        chal_index: Index,
        block: AgBlockReader<'a>,
        block_proof: Vec<[u8; 32]>,
    ) -> Result<(), Error> {
        if self.new_state.account_root().raw_data() != reverted_account_root {
            return Err(Error::InvalidNewAccountRoot);
        }
        // generate a new block to instead the invalid one
        let new_block = AgBlock::new_reverted_block(
            block,
            reverted_account_root,
            chal_index,
        );
        let block_number: u64 = block.number().unpack();
        let block_hash = {
            let mut hasher = new_blake2b();
            hasher.update(new_block.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            hash
        };
        let block_root = compute_block_root(
            vec![(block_number as usize, block_hash)],
            block_number + 1,
            block_proof,
        )
        .map_err(|_| Error::InvalidBlockMerkleProof)?;
        // verify global state
        let expected_state = self
            .old_state
            .to_entity()
            .as_builder()
            .account_root(reverted_account_root.pack())
            .block_root(block_root.pack())
            .build();
        if expected_state.as_slice() != self.new_state.as_slice() {
            return Err(Error::InvalidGlobalState);
        }
        Ok(())
    }

    /// Invalid a block
    /// 1. proof that block/txs/accounts are actually exists
    /// 2. run txs, compare the state to the block's account_root
    /// 3. generate a reverted block to instead the invalid block
    /// 4. put a penalize tx in reverted block
    /// 5. verify new account root and block root
    pub fn verify(&self) -> Result<(), Error> {
        // load challenge cell
        let challenge_cell_data_hash: [u8; 32] = self.action.challenge_cell_data_hash().unpack();
        let (challenge_cell_index, challenge_context) =
            load_challenge_context(&challenge_cell_data_hash)?.expect("can't find challenge cell");
        let challenge_context_reader = challenge_context.as_reader();
        let block = challenge_context_reader.block();
        self.verify_challenge_cell(challenge_cell_index)?;

        // load aggregator and challenge
        let ag_index: Index = block.ag_index().unpack();
        let chal_index: Index = challenge_context_reader.challenger_index().unpack();
        let ag_kv: KVMap = self.action.aggregator_token_kv().unpack();
        let chal_kv: KVMap = self.action.challenger_token_kv().unpack();
        let ag_account = self.action.ag_account();
        let chal_account = self.action.challenger_account();

        if ag_account.index().unpack() != ag_index {
            return Err(Error::InvalidAggregatorIndex);
        }
        if chal_account.index().unpack() != chal_index {
            return Err(Error::InvalidChallengerIndex);
        }

        // load account proof
        let proof = self.action.accounts_proof();
        let leaves_path = proof.leaves_path().unpack();
        let merkle_branches: Vec<(smt::H256, u8)> =
            Unpack::<Vec<([u8; 32], u8)>>::unpack(&proof.proof())
                .into_iter()
                .map(|(node, height)| (node.into(), height))
                .collect();

        // load block proof
        let block_proof: Vec<[u8; 32]> = self
            .action
            .block_proof()
            .iter()
            .map(|item| item.unpack())
            .collect();

        // verification
        self.verify_block(block, block_proof.clone())?;
        self.verify_accounts(
            ag_account,
            ag_kv.clone(),
            chal_account,
            chal_kv.clone(),
            block,
            leaves_path.clone(),
            merkle_branches.clone(),
        )?;
        let reverted_account_root = calculate_reverted_account_root(
            ag_account,
            ag_kv,
            chal_account,
            chal_kv,
            leaves_path,
            merkle_branches,
        )?;
        self.verify_reverted_state(reverted_account_root, chal_index, block, block_proof)?;
        Ok(())
    }
}

fn load_challenge_context(
    challenge_cell_data_hash: &[u8],
) -> Result<Option<(usize, ChallengeContext)>, Error> {
    const BUF_LEN: usize = 4096;

    for i in 0.. {
        match syscalls::load_cell_by_field(HASH_SIZE, 0, i, Source::Input, CellField::DataHash) {
            Ok(data_hash) if &data_hash[..] == challenge_cell_data_hash => {
                let buf =
                    syscalls::load_cell_data(BUF_LEN, 0, i, Source::Input).expect("load cell data");
                let challenge_context = match ChallengeContextReader::verify(&buf, false) {
                    Ok(()) => ChallengeContext::new_unchecked(buf.into()),
                    Err(_) => return Err(Error::InvalidChallengeContext),
                };
                return Ok(Some((i, challenge_context)));
            }
            Ok(_data_hash) => continue,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => panic!("syscall err: {:?}", err),
        }
    }

    Ok(None)
}

fn accounts_to_merkle_leaves<'a>(
    items: &[(AccountReader<'a>, KVMap)],
) -> Vec<(smt::H256, smt::H256)> {
    let mut leaves: Vec<_> = Vec::with_capacity(items.len() * 2);
    for (account, kv) in items {
        let index: Index = account.index().unpack();
        for (k, v) in kv {
            leaves.push((smt::token_id_key(index, k), Value::from(*v).to_h256()));
        }
        let value = Value::from(account.to_entity());
        leaves.push((smt::account_index_key(index.into()), value.to_h256()));
    }
    leaves
}

pub fn calculate_reverted_account_root<'a>(
    ag_account: AccountReader<'a>,
    mut ag_kv: KVMap,
    chal_account: AccountReader<'a>,
    mut chal_kv: KVMap,
    leaves_path: Vec<Vec<u8>>,
    merkle_branches: Vec<(smt::H256, u8)>,
) -> Result<[u8; 32], Error> {
    // calculate reward
    let reward_amount = {
        let balance: u64 = ag_kv.get(&CKB_TOKEN_ID).map(|b| *b).unwrap_or(0);
        balance.saturating_mul(CHALLENGE_REWARD_RATE.0) / CHALLENGE_REWARD_RATE.1
    };
    let chal_balance: u64 = chal_kv.get(&CKB_TOKEN_ID).map(|b| *b).unwrap_or(0);

    ag_kv.insert(CKB_TOKEN_ID, 0);
    chal_kv.insert(
        CKB_TOKEN_ID,
        chal_balance.checked_add(reward_amount).expect("no overflow"),
    );

    let leaves = accounts_to_merkle_leaves(&[(ag_account, ag_kv), (chal_account, chal_kv)]);
    let root = compute_root_with_proof(leaves, leaves_path, merkle_branches)
        .map_err(|_| Error::InvalidAccountMerkleProof)?
        .into();
    Ok(root)
}
