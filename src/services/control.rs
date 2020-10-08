use crate::grpc::control::*;
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Control: Debug + Send + Sync + 'static {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()>;
    async fn get_status(&self) -> anyhow::Result<StatusData>;
}
