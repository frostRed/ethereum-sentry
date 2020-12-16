use super::*;
use anyhow::bail;

#[derive(Debug)]
pub struct DummyControl;

#[async_trait]
impl Control for DummyControl {
    async fn forward_inbound_message(&self, _: InboundMessage) -> anyhow::Result<()> {
        Ok(())
    }
    async fn get_status_data(&self) -> anyhow::Result<StatusData> {
        bail!("Not implemented")
    }
}
