use crate::eth::StatusData;
use anyhow::{anyhow, bail, Context};
use async_trait::async_trait;
use auto_impl::auto_impl;
use ethereum::{Header, Transaction, TransactionAction, TransactionSignature};
use ethereum_types::{H256, U64};
use futures::stream::{BoxStream, FuturesOrdered};
use rlp::{Decodable, DecoderError, Encodable, Rlp};
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::Deserialize;
use serde_json::json;
use std::fmt::Debug;
use tokio_compat_02::FutureExt;
use tokio_stream::StreamExt;
use tracing::*;

mod dummy;
pub use dummy::*;

mod tarpc;
pub use self::tarpc::*;

mod web3;
pub use self::web3::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockId {
    Hash(H256),
    Number(u64),
}

impl Decodable for BlockId {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.size() == 32 {
            Ok(Self::Hash(rlp.as_val()?))
        } else {
            Ok(Self::Number(rlp.as_val()?))
        }
    }
}

impl Encodable for BlockId {
    fn rlp_append(&self, s: &mut rlp::RlpStream) {
        match self {
            Self::Hash(v) => Encodable::rlp_append(v, s),
            Self::Number(v) => Encodable::rlp_append(v, s),
        }
    }
}

impl From<BlockId> for ::web3::types::BlockId {
    fn from(id: BlockId) -> Self {
        match id {
            BlockId::Hash(hash) => Self::Hash(hash),
            BlockId::Number(number) => U64::from(number).into(),
        }
    }
}

#[derive(Clone, Debug, RlpEncodable, RlpDecodable)]
pub struct GetBlockHeaders {
    pub block: BlockId,
    pub max_headers: u64,
    pub skip: u64,
    pub reverse: bool,
}

#[derive(Clone, Debug, RlpEncodable, RlpDecodable)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    pub ommers: Vec<Header>,
}

/// Provider of Ethereum blockchain data.
#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait DataProvider: Debug + Send + Sync + 'static {
    async fn get_status_data(&self) -> anyhow::Result<StatusData>;
    async fn resolve_block_height(&self, block: H256) -> anyhow::Result<Option<u64>>;
    fn get_block_headers(&self, blocks: Vec<BlockId>) -> BoxStream<anyhow::Result<Header>>;
    fn get_block_bodies(&self, block: Vec<H256>) -> BoxStream<anyhow::Result<BlockBody>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blockid_rlp() {
        for &id in &[
            BlockId::Hash(H256::random()),
            BlockId::Number(rand::random()),
        ] {
            assert_eq!(id, rlp::decode(&rlp::encode(&id)).unwrap());
        }
    }
}
