use super::DataProvider;
use anyhow::bail;
use async_trait::async_trait;
use ethereum_tarpc_api::*;
use std::fmt::Display;
use stubborn_io::{ReconnectOptions, StubbornTcpStream};
use tarpc::{client::Config, serde_transport::Transport};
use tokio_serde::formats::*;
use tokio_stream::empty;

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
        bail!("not implemented")
    }

    async fn resolve_block_height(&self, _: ethereum_types::H256) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }

    fn get_block_headers(
        &self,
        _: Vec<super::BlockId>,
    ) -> futures::stream::BoxStream<anyhow::Result<ethereum::Header>> {
        Box::pin(empty())
    }

    fn get_block_bodies(
        &self,
        _: Vec<ethereum_types::H256>,
    ) -> futures::stream::BoxStream<anyhow::Result<super::BlockBody>> {
        Box::pin(empty())
    }
}
