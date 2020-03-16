#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(alloc_error_handler)]
#![feature(panic_info_message)]

//! Challenge contract
//! 1. anyone can start a challenge cell with this script as type, with a small bond
//! 2. a challenge cell stores ChallengeContext in data, and provide ChallengeProof to verify the ChallengeContext.
//! 3. anyone can respond a challnege by provides ChallengeRepond proof, if respond success the challenge cell and bond is unlocked.
//! 4. after `CHALLENGE_PREPARE_TIMEOUT`, the challenge cell can revert the block that described in the ChallengeContext.

use alloc::vec::Vec;
use ckb_contract_std::{ckb_constants::*, setup, since, syscalls};
use godwoken_executor::{executor::Executor, state::State};
use godwoken_types::{
    cache::{KVMap, TxWithHash},
    core::Index,
    packed::*,
    prelude::*,
};
use godwoken_utils::{
    hash::new_blake2b,
    mmr::compute_tx_root,
    smt::{self, compute_root_with_proof, Value, ValueTrait},
};

const BUF_LEN: usize = 4096;
const HASH_LEN: usize = 32;
/// must wait WITHDRAW_WAIT_EPOCHS epochs before withdraw challnege
const WITHDRAW_WAIT_EPOCHS: u64 = 6;

#[repr(i8)]
enum Error {
    InvalidEncoding = -1,
    NoUnlockCell = -2,
    InvalidMerkleProof = -3,
    InvalidSince = -4,
}

#[no_mangle]
fn main() -> i8 {
    match contract_entry() {
        Ok(()) => 0,
        Err(err) => err as i8,
    }
}

setup!(main);

fn contract_entry() -> Result<(), Error> {
    let args = load_challenge_args()?;
    if let Ok(_) =
        syscalls::load_cell_by_field(HASH_LEN, 0, 0, Source::GroupOutput, CellField::TypeHash)
    {
        // create a challenge cell
        verify_challenge_context()?;
        return Ok(());
    }

    // destroy the challenge cell
    let buf = syscalls::load_witness(BUF_LEN, 0, 0, Source::GroupInput).expect("load witness");
    let respond = match ChallengeRespondReader::verify(&buf, false) {
        Ok(()) => ChallengeRespond::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    match respond.as_reader().to_enum() {
        ChallengeRespondUnionReader::WithdrawChallenge(_withdraw) => {
            verify_withdraw_challenge(args.as_reader())
        }
        ChallengeRespondUnionReader::InvalidChallenge(invalid_challenge) => {
            verify_invalid_challenge(invalid_challenge)
        }
    }
}

/// load challnege args
fn load_challenge_args() -> Result<ChallengeArgs, Error> {
    let buf = syscalls::load_script(BUF_LEN, 0).expect("load script");
    let script = match ScriptReader::verify(&buf, false) {
        Ok(()) => Script::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    let buf: Vec<u8> = script.args().unpack();
    let args = match ChallengeArgsReader::verify(&buf, false) {
        Ok(()) => ChallengeArgs::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    Ok(args)
}

fn verify_withdraw_challenge<'a>(args: ChallengeArgsReader<'a>) -> Result<(), Error> {
    const SINCE_LEN: usize = 8;
    // verify withdraw time
    let buf = syscalls::load_input_by_field(SINCE_LEN, 0, 0, Source::GroupInput, InputField::Since)
        .map_err(|_| Error::InvalidSince)?;
    let withdraw_since = {
        let mut raw_since = [0u8; 8];
        raw_since.copy_from_slice(&buf);
        since::Since::new(u64::from_le_bytes(raw_since))
    };
    if !withdraw_since.is_relative() {
        return Err(Error::InvalidSince);
    }
    let withdraw_epoch = withdraw_since
        .extract_lock_value()
        .and_then(|value| value.epoch())
        .ok_or(Error::InvalidSince)?;
    if withdraw_epoch.number() < WITHDRAW_WAIT_EPOCHS {
        return Err(Error::InvalidSince);
    }
    // verify inputs include withdraw lock hash
    let withdraw_lock_hash = args.withdraw_lock_hash();
    for i in 0.. {
        let buf = match syscalls::load_cell_by_field(
            HASH_LEN,
            0,
            i,
            Source::GroupInput,
            CellField::LockHash,
        ) {
            Ok(buf) => buf,
            Err(SysError::ItemMissing) => continue,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => panic!("syscall error: {:?}", err),
        };
        if withdraw_lock_hash.as_slice() == &buf[..] {
            return Ok(());
        }
    }
    return Err(Error::NoUnlockCell);
}

fn verify_invalid_challenge<'a>(
    invalid_challenge: InvalidChallengeReader<'a>,
) -> Result<(), Error> {
    // load challenge context
    let buf = syscalls::load_cell_data(BUF_LEN, 0, 0, Source::GroupInput).expect("load data");
    let context = match ChallengeContextReader::verify(&buf, false) {
        Ok(()) => ChallengeContext::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    let context_reader = context.as_reader();
    let block = context_reader.block();
    // initialize state from touched accounts
    let mut state = State::new(
        invalid_challenge
            .touched_accounts()
            .iter()
            .zip(invalid_challenge.touched_accounts_token_kv().iter())
            .map(|(account, kv)| {
                let kv: KVMap = kv.unpack();
                (account, kv)
            })
            .collect(),
    );
    // extract account proof
    let proof = invalid_challenge.touched_accounts_proof();
    let leaves_path = proof.leaves_path().unpack();
    let merkle_branches: Vec<(smt::H256, u8)> =
        Unpack::<Vec<([u8; 32], u8)>>::unpack(&proof.proof())
            .into_iter()
            .map(|(node, height)| (node.into(), height))
            .collect();
    // verify prev account root
    let leaves = state_to_merkle_leaves(&state);
    let calculated_root: [u8; 32] =
        compute_root_with_proof(leaves, leaves_path.clone(), merkle_branches.clone())
            .map_err(|_| Error::InvalidMerkleProof)?
            .into();
    if &calculated_root != block.prev_account_root().raw_data() {
        return Err(Error::InvalidMerkleProof);
    }
    // verify new state
    let executor = Executor::new();
    let ag_index: Index = block.ag_index().unpack();
    let txs = context_reader.txs();
    let tx_with_hashes = build_tx_hashes(&txs);
    for tx in tx_with_hashes {
        if executor.run(&mut state, tx, ag_index).is_err() {
            // errors occured, represents the block is invalid
            return Ok(());
        }
    }
    // check new account root
    let leaves: Vec<_> = state_to_merkle_leaves(&state);
    let calculated_root: [u8; 32] = compute_root_with_proof(leaves, leaves_path, merkle_branches)
        .map_err(|_| Error::InvalidMerkleProof)?
        .into();
    if &calculated_root != block.account_root().raw_data() {
        return Err(Error::InvalidMerkleProof);
    }
    // invalid challenge
    Ok(())
}

fn verify_challenge_context() -> Result<(), Error> {
    // load challenge context
    let buf = syscalls::load_cell_data(BUF_LEN, 0, 0, Source::GroupOutput).expect("load data");
    let context = match ChallengeContextReader::verify(&buf, false) {
        Ok(()) => ChallengeContext::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    // load challenge proof
    let buf = syscalls::load_witness(BUF_LEN, 0, 0, Source::GroupOutput).expect("load witness");
    let wit_args = match WitnessArgsReader::verify(&buf, false) {
        Ok(()) => WitnessArgs::new_unchecked(buf.into()),
        Err(_) => return Err(Error::InvalidEncoding),
    };
    let proof = wit_args
        .output_type()
        .to_opt()
        .ok_or(Error::InvalidEncoding)
        .and_then(|buf| {
            let buf: Vec<u8> = buf.unpack();
            match ChallengeProofReader::verify(&buf, false) {
                Ok(()) => Ok(ChallengeProof::new_unchecked(buf.into())),
                Err(_) => Err(Error::InvalidEncoding),
            }
        })?;
    // verify challenge context
    let context_reader = context.as_reader();
    let proof_reader = proof.as_reader();
    // verify tx_root
    let block = context_reader.block();
    let txs = context_reader.txs();
    let tx_with_hashes = build_tx_hashes(&txs);
    let leaves: Vec<_> = {
        tx_with_hashes
            .into_iter()
            .enumerate()
            .map(|(i, tx)| (i, tx.tx_hash))
            .collect()
    };
    let txs_count: u32 = block.txs_count().unpack();
    let txs_proof: Vec<[u8; 32]> = proof_reader
        .txs_proof()
        .iter()
        .map(|item| item.unpack())
        .collect();
    let calculated_tx_root =
        compute_tx_root(leaves, txs_count, txs_proof).map_err(|_| Error::InvalidMerkleProof)?;
    if &calculated_tx_root != block.tx_root().raw_data() {
        return Err(Error::InvalidMerkleProof);
    }
    Ok(())
}

fn state_to_merkle_leaves(state: &State) -> Vec<(smt::H256, smt::H256)> {
    // verify account and kv
    let mut leaves: Vec<_> = Vec::with_capacity(state.len() * 2);
    for (account, kv) in state.iter() {
        let index: Index = account.index().unpack();
        for (k, v) in kv {
            leaves.push((smt::token_id_key(index, k), Value::from(*v).to_h256()));
        }
        let value = Value::from(account.clone());
        leaves.push((smt::account_index_key(index.into()), value.to_h256()));
    }
    leaves
}

fn build_tx_hashes<'a>(txs: &'a TxVecReader<'a>) -> Vec<TxWithHash<'a>> {
    txs.iter()
        .map(|tx| {
            let mut hasher = new_blake2b();
            hasher.update(tx.as_slice());
            let mut hash = [0u8; 32];
            hasher.finalize(&mut hash);
            TxWithHash {
                raw: tx,
                tx_hash: hash.clone(),
            }
        })
        .collect()
}
