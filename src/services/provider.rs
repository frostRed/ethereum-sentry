use async_trait::async_trait;
use ethereum::{Header, Transaction};
use ethereum_types::H256;
use rlp::{Decodable, DecoderError, Encodable, Rlp};
use rlp_derive::{RlpDecodable, RlpEncodable};
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
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
pub trait DataProvider: Debug + Send + Sync + 'static {
    async fn resolve_block_height(&self, block: H256) -> anyhow::Result<Option<u64>>;
    async fn get_block_headers(
        &self,
        blocks: Vec<BlockId>,
    ) -> anyhow::Result<Vec<ethereum::Header>>;
    async fn get_block_bodies(&self, block: Vec<H256>) -> anyhow::Result<Vec<BlockBody>>;
}

#[derive(Debug)]
pub struct DummyDataProvider;

#[async_trait]
impl DataProvider for DummyDataProvider {
    async fn resolve_block_height(&self, _: H256) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }

    async fn get_block_headers(&self, _: Vec<BlockId>) -> anyhow::Result<Vec<ethereum::Header>> {
        Ok(vec![])
    }
    async fn get_block_bodies(&self, _: Vec<H256>) -> anyhow::Result<Vec<BlockBody>> {
        Ok(vec![])
    }
}
