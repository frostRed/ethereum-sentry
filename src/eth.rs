use std::{collections::BTreeSet, ops::Add};

use arrayvec::ArrayString;
use devp2p::*;
use enum_primitive_derive::*;
use ethereum_forkid::{ForkHash, ForkId};
use ethereum_types::*;
use rlp_derive::*;
use serde::Deserialize;

pub fn capability_name() -> CapabilityName {
    CapabilityName(ArrayString::from("eth").unwrap())
}

#[derive(Clone, Debug, RlpEncodable, RlpDecodable)]
pub struct StatusMessage {
    pub protocol_version: usize,
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub genesis_hash: H256,
    pub fork_id: ForkId,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Forks {
    pub genesis: H256,
    pub passed: BTreeSet<u64>,
    pub next: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct StatusData {
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub fork_data: Forks,
}

impl From<StatusData> for StatusMessage {
    fn from(value: StatusData) -> Self {
        Self {
            protocol_version: 64,
            network_id: value.network_id,
            total_difficulty: value.total_difficulty,
            best_hash: value.best_hash,
            genesis_hash: value.fork_data.genesis,
            fork_id: ForkId {
                hash: value
                    .fork_data
                    .passed
                    .into_iter()
                    .filter(|&fork_block| fork_block != 0)
                    .fold(ForkHash::from(value.fork_data.genesis), Add::add),
                next: value.fork_data.next.unwrap_or(0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Primitive)]
pub enum MessageId {
    Status = 0,
    NewBlockHashes = 1,
    Transactions = 2,
    GetBlockHeaders = 3,
    BlockHeaders = 4,
    GetBlockBodies = 5,
    BlockBodies = 6,
    NewBlock = 7,
    NewPooledTransactionHashes = 8,
    GetPooledTransactions = 9,
    PooledTransactions = 10,
    GetNodeData = 13,
    NodeData = 14,
    GetReceipts = 15,
    Receipts = 16,
}
