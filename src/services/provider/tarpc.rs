use super::DataProvider;
use anyhow::{bail, Context};
use async_trait::async_trait;
use ethereum_tarpc_api::*;
use futures::stream::FuturesOrdered;
use std::fmt::Display;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{client::Config, context, serde_transport::Transport};
use tokio_serde::formats::*;
use tracing::*;

#[derive(Debug)]
pub struct TarpcDataProvider {
    client: EthApiClient,
}

impl TarpcDataProvider {
    pub async fn new(addr: impl Display) -> anyhow::Result<Self> {
        let reconnect_opts = ReconnectOptions::new().with_exit_if_first_connect_fails(false);
        let tcp_stream =
            StubbornTcpStream::connect_with_options(addr.to_string(), reconnect_opts).await?;
        let transport = Transport::from((tcp_stream, Bincode::default()));
        Ok(Self {
            client: EthApiClient::new(Config::default(), transport).spawn()?,
        })
    }
}

#[async_trait]
impl DataProvider for TarpcDataProvider {
    async fn get_status_data(&self) -> anyhow::Result<crate::eth::StatusData> {
        let mut client = self.client.clone();

        trace!("Getting fork data");
        let forks = client
            .forks(context::current())
            .await?
            .map_err(anyhow::Error::msg)?;

        trace!("Getting best hash");
        let (best_hash, _) = client
            .best_block(context::current())
            .await?
            .map_err(anyhow::Error::msg)?;

        trace!("Getting total difficulty");
        let total_difficulty = client
            .total_difficulty(context::current(), best_hash)
            .await?
            .map_err(anyhow::Error::msg)?
            .context("no total difficulty")?;

        Ok(crate::eth::StatusData {
            network_id: 1,
            total_difficulty,
            best_hash,
            fork_data: crate::eth::Forks {
                genesis: forks.genesis,
                forks: forks.forks,
            },
        })
    }

    async fn resolve_block_height(
        &self,
        hash: ethereum_types::H256,
    ) -> anyhow::Result<Option<u64>> {
        Ok(self
            .client
            .clone()
            .header(context::current(), hash)
            .await?
            .map_err(anyhow::Error::msg)?
            .map(|header| header.number.as_u64()))
    }

    fn get_block_headers(
        &self,
        ids: Vec<super::BlockId>,
    ) -> futures::stream::BoxStream<anyhow::Result<ethereum::Header>> {
        Box::pin(
            ids.into_iter()
                .map(|id| async move {
                    let mut client = self.client.clone();

                    if let Some(hash) = match id {
                        super::BlockId::Hash(hash) => Some(hash),
                        super::BlockId::Number(number) => client
                            .canonical_hash(context::current(), number)
                            .await?
                            .map_err(anyhow::Error::msg)?,
                    } {
                        return client
                            .header(context::current(), hash)
                            .await?
                            .map_err(anyhow::Error::msg)?
                            .context("not found");
                    }

                    bail!("not found")
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }

    fn get_block_bodies(
        &self,
        ids: Vec<ethereum_types::H256>,
    ) -> futures::stream::BoxStream<anyhow::Result<super::BlockBody>> {
        Box::pin(
            ids.into_iter()
                .map(|hash| async move {
                    let body = self
                        .client
                        .clone()
                        .body(context::current(), hash)
                        .await?
                        .map_err(anyhow::Error::msg)?
                        .context("not found")?;

                    Ok(super::BlockBody {
                        transactions: body.transactions,
                        ommers: body.ommers,
                    })
                })
                .collect::<FuturesOrdered<_>>(),
        )
    }
}
