use super::*;

#[derive(Debug)]
pub struct DummyDataProvider;

#[async_trait]
impl DataProvider for DummyDataProvider {
    async fn get_status_data(&self) -> anyhow::Result<StatusData> {
        bail!("Not implemented")
    }

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
