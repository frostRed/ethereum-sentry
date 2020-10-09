use enum_primitive_derive::*;
use ethereum_types::*;
use rlp_derive::*;

#[derive(Clone, Debug, RlpEncodable, RlpDecodable)]
pub struct StatusMessage {
    pub protocol_version: usize,
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub genesis_hash: H256,
}

#[derive(Clone, Debug)]
pub struct StatusData {
    pub network_id: u64,
    pub total_difficulty: U256,
    pub best_hash: H256,
    pub genesis_hash: H256,
}

impl From<StatusData> for StatusMessage {
    fn from(value: StatusData) -> Self {
        Self {
            protocol_version: 63,
            network_id: value.network_id,
            total_difficulty: value.total_difficulty,
            best_hash: value.best_hash,
            genesis_hash: value.genesis_hash,
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
