use crate::{
    eth::StatusData,
    grpc::control::{control_client::ControlClient, *},
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::fmt::Debug;
use tonic::transport::Channel;

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait Control: Debug + Send + Sync + 'static {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()>;
    async fn get_status_data(&self) -> anyhow::Result<StatusData>;
}

#[derive(Debug)]
pub struct GrpcControl {
    client: ControlClient<Channel>,
}

impl GrpcControl {
    pub async fn connect(addr: String) -> anyhow::Result<Self> {
        Ok(Self {
            client: ControlClient::connect(addr).await?,
        })
    }
}

#[async_trait]
impl Control for GrpcControl {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()> {
        self.client.clone().forward_inbound_message(message).await?;

        Ok(())
    }

    async fn get_status_data(&self) -> anyhow::Result<StatusData> {
        let status_data = self.client.clone().get_status(()).await?.into_inner();

        Ok(StatusData {
            network_id: status_data.network_id,
            total_difficulty: hex::encode(status_data.total_difficulty).parse()?,
            best_hash: hex::encode(status_data.best_hash).parse()?,
            genesis_hash: hex::encode(status_data.genesis_hash).parse()?,
        })
    }
}
