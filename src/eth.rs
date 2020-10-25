use std::collections::BTreeSet;

use arrayvec::ArrayString;
use devp2p::*;
use enum_primitive_derive::*;
use ethereum_forkid::ForkId;
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
    pub forks: BTreeSet<u64>,
}

#[derive(Clone, Debug)]
pub struct StatusData {
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub fork_data: Forks,
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
