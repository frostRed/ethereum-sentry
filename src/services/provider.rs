use anyhow::{anyhow, ensure, Context};
use async_trait::async_trait;
use ethereum::{Header, Transaction};
use ethereum_types::{H256, U64};
use futures::{
    stream::{BoxStream, FuturesUnordered},
    TryStreamExt,
};
use rlp::{Decodable, DecoderError, Encodable, Rlp};
use rlp_derive::{RlpDecodable, RlpEncodable};
use std::fmt::Debug;

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

impl From<BlockId> for web3::types::BlockId {
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
pub trait DataProvider: Debug + Send + Sync + 'static {
    async fn resolve_block_height(&self, block: H256) -> anyhow::Result<Option<u64>>;
    fn get_block_headers(&self, blocks: Vec<BlockId>) -> BoxStream<anyhow::Result<Header>>;
    fn get_block_bodies(&self, block: Vec<H256>) -> BoxStream<anyhow::Result<BlockBody>>;
}

#[async_trait]
impl<T: DataProvider + ?Sized> DataProvider for Box<T> {
    async fn resolve_block_height(&self, block: H256) -> anyhow::Result<Option<u64>> {
        (**self).resolve_block_height(block).await
    }

    fn get_block_headers(&self, blocks: Vec<BlockId>) -> BoxStream<anyhow::Result<Header>> {
        (**self).get_block_headers(blocks)
    }

    fn get_block_bodies(&self, block: Vec<H256>) -> BoxStream<anyhow::Result<BlockBody>> {
        (**self).get_block_bodies(block)
    }
}

#[derive(Debug)]
pub struct DummyDataProvider;

#[async_trait]
impl DataProvider for DummyDataProvider {
    async fn resolve_block_height(&self, _: H256) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }

    fn get_block_headers(&self, _: Vec<BlockId>) -> BoxStream<anyhow::Result<Header>> {
        Box::pin(futures::stream::iter(vec![]))
    }
    fn get_block_bodies(&self, _: Vec<H256>) -> BoxStream<anyhow::Result<BlockBody>> {
        Box::pin(futures::stream::iter(vec![]))
    }
}

#[derive(Debug)]
pub struct Web3DataProvider {
    client: web3::Web3<web3::transports::Http>,
}

impl Web3DataProvider {
    pub fn new(addr: String) -> anyhow::Result<Self> {
        let transport = web3::transports::Http::new(&addr)?;
        let client = web3::Web3::new(transport);

        Ok(Self { client })
    }
}

fn web3_block_to_header<TX>(block: web3::types::Block<TX>) -> Option<Header> {
    Some(Header {
        parent_hash: block.parent_hash,
        ommers_hash: block.uncles_hash,
        beneficiary: block.author,
        state_root: block.state_root,
        transactions_root: block.transactions_root,
        receipts_root: block.receipts_root,
        logs_bloom: block.logs_bloom?,
        difficulty: block.difficulty,
        number: block.number?.as_u64().into(),
        gas_limit: block.gas_limit,
        gas_used: block.gas_used,
        timestamp: block.timestamp.as_u64(),
        extra_data: block.extra_data.0,
        mix_hash: block.mix_hash?,
        nonce: block.nonce?,
    })
}

#[async_trait]
impl DataProvider for Web3DataProvider {
    async fn resolve_block_height(&self, block: H256) -> anyhow::Result<Option<u64>> {
        Ok(self
            .client
            .eth()
            .block(block.into())
            .await?
            .and_then(|block| block.number)
            .map(|v| v.as_u64()))
    }

    fn get_block_headers(&self, blocks: Vec<BlockId>) -> BoxStream<anyhow::Result<Header>> {
        Box::pin(
            blocks
                .into_iter()
                .map(|block| async move {
                    let block = self
                        .client
                        .eth()
                        .block(block.into())
                        .await?
                        .ok_or_else(|| anyhow!("Block not found"))?;

                    web3_block_to_header(block).ok_or_else(|| anyhow!("Pending block"))
                })
                .collect::<FuturesUnordered<_>>(),
        )
    }

    fn get_block_bodies(&self, blocks: Vec<H256>) -> BoxStream<anyhow::Result<BlockBody>> {
        Box::pin(
            blocks
                .into_iter()
                .map(|id| async move {
                    let block = self
                        .client
                        .eth()
                        .block_with_txs(id.into())
                        .await?
                        .ok_or_else(|| anyhow!("Block not found"))?;

                    let header = web3_block_to_header(block.clone())
                        .ok_or_else(|| anyhow!("Pending block?"))?;

                    let ommers = block
                        .uncles
                        .iter()
                        .map(|&uncle_hash| async move {
                            web3_block_to_header(
                                self.client
                                    .eth()
                                    .block(uncle_hash.into())
                                    .await?
                                    .ok_or_else(|| anyhow!("Uncle not found"))?,
                            )
                            .ok_or_else(|| anyhow!("Pending block?"))
                        })
                        .collect::<FuturesUnordered<_>>()
                        .try_collect()
                        .await
                        .context("Failed to fetch uncles")?;

                    let transactions = block
                        .transactions
                        .iter()
                        .map(|tx| {
                            Ok(rlp::decode(
                                &tx.raw
                                    .as_ref()
                                    .ok_or_else(|| anyhow!("Where's the raw tx data?"))?
                                    .0,
                            )?)
                        })
                        .collect::<anyhow::Result<_>>()?;

                    let assembled_block = ethereum::Block::new(header.into(), transactions, ommers);
                    ensure!(
                        id == assembled_block.header.hash(),
                        "Hash mismatch: expected {}, found {}",
                        id,
                        assembled_block.header.hash()
                    );

                    let ethereum::Block {
                        transactions,
                        ommers,
                        ..
                    } = assembled_block;

                    Ok(BlockBody {
                        transactions,
                        ommers,
                    })
                })
                .collect::<FuturesUnordered<_>>(),
        )
    }
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
