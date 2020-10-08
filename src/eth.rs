use crate::grpc::control::StatusData;
use enum_primitive_derive::*;
use ethereum_types::*;
use rlp_derive::*;
use std::convert::TryFrom;

#[derive(Clone, Debug, RlpEncodable, RlpDecodable)]
pub struct StatusMessage {
    pub protocol_version: usize,
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub genesis_hash: H256,
}

impl TryFrom<StatusData> for StatusMessage {
    type Error = anyhow::Error;

    fn try_from(value: StatusData) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol_version: 63,
            network_id: value.network_id,
            total_difficulty: hex::encode(value.total_difficulty).parse()?,
            best_hash: hex::encode(value.best_hash).parse()?,
            genesis_hash: hex::encode(value.genesis_hash).parse()?,
        })
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
