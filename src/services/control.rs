use crate::{
    eth::{Forks, StatusData},
    grpc::control::{control_client::ControlClient, *},
};
use anyhow::anyhow;
use async_trait::async_trait;
use auto_impl::auto_impl;
use std::fmt::Debug;
use tokio_compat_02::FutureExt;
use tonic::transport::Channel;

mod dummy;
pub use dummy::*;

mod grpc;
pub use grpc::*;

#[async_trait]
#[auto_impl(&, Box, Arc)]
pub trait Control: Debug + Send + Sync + 'static {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()>;
    async fn get_status_data(&self) -> anyhow::Result<StatusData>;
}
