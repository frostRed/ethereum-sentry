use crate::{eth::StatusData, grpc::control::*};
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::fmt::Debug;

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait Control: Debug + Send + Sync + 'static {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()>;
    async fn get_status_data(&self) -> anyhow::Result<StatusData>;
}
