use std::{fmt::Debug, sync::Arc};

use super::{Control, DataProvider};
use crate::{eth::*, grpc::sentry::sentry_server::*, CapabilityServerImpl};
use async_trait::async_trait;
use bytes::Bytes;
use devp2p::*;
use futures::stream::FuturesUnordered;
use num_traits::ToPrimitive;
use tokio::stream::StreamExt;
use tonic::Response;

#[derive(Clone, Debug)]
pub struct SentryService<C, DP>
where
    C: Debug,
    DP: Debug,
{
    capability_server: Arc<CapabilityServerImpl<C, DP>>,
}

impl<C, DP> SentryService<C, DP>
where
    C: Debug,
    DP: Debug,
{
    pub fn new(capability_server: Arc<CapabilityServerImpl<C, DP>>) -> Self {
        Self { capability_server }
    }
}

impl<C, DP> SentryService<C, DP>
where
    C: Control,
    DP: DataProvider,
{
    async fn send_by_predicate<F, IT>(
        &self,
        request: Option<crate::grpc::sentry::OutboundMessageData>,
        pred: F,
    ) where
        F: FnOnce(&CapabilityServerImpl<C, DP>) -> IT,
        IT: IntoIterator<Item = PeerId>,
    {
        if let Some(data) = request {
            if let Some(id) = MessageId::from_outbound_message_id(data.id) {
                let data = Bytes::from(data.data);
                let id = id.to_usize().unwrap();

                (pred)(&*self.capability_server)
                    .into_iter()
                    .map(|peer| {
                        let data = data.clone();
                        async move {
                            if let Some(mut sender) = self.capability_server.sender(peer) {
                                let _ = sender
                                    .send(OutboundEvent::Message {
                                        capability_name: capability_name(),
                                        message: Message { id, data },
                                    })
                                    .await;
                            }
                        }
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<()>()
                    .await;
            }
        }
    }
}

#[async_trait]
impl<C, DP> Sentry for SentryService<C, DP>
where
    C: Control,
    DP: DataProvider,
{
    async fn penalize_peer(
        &self,
        request: tonic::Request<crate::grpc::sentry::PenalizePeerRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let peer = hex::encode(&request.into_inner().peer_id)
            .parse::<PeerId>()
            .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;
        if let Some(mut sender) = self.capability_server.sender(peer) {
            let _ = sender
                .send(OutboundEvent::Disconnect {
                    reason: DisconnectReason::DisconnectRequested,
                })
                .await;
        }

        Ok(Response::new(()))
    }

    async fn send_message_by_min_block(
        &self,
        request: tonic::Request<crate::grpc::sentry::SendMessageByMinBlockRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let crate::grpc::sentry::SendMessageByMinBlockRequest { data, min_block } =
            request.into_inner();
        self.send_by_predicate(data, |capability_server| {
            capability_server.peers_with_min_block(min_block)
        })
        .await;

        Ok(Response::new(()))
    }

    async fn send_message_by_id(
        &self,
        request: tonic::Request<crate::grpc::sentry::SendMessageByIdRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let crate::grpc::sentry::SendMessageByIdRequest { peer_id, data } = request.into_inner();

        let peer = hex::encode(&peer_id)
            .parse::<PeerId>()
            .map_err(|e| tonic::Status::invalid_argument(e.to_string()))?;

        self.send_by_predicate(data, |_| std::iter::once(peer))
            .await;

        Ok(Response::new(()))
    }

    async fn send_message_to_random_peers(
        &self,
        request: tonic::Request<crate::grpc::sentry::SendMessageToRandomPeersRequest>,
    ) -> Result<Response<()>, tonic::Status> {
        let crate::grpc::sentry::SendMessageToRandomPeersRequest { max_peers, data } =
            request.into_inner();

        self.send_by_predicate(data, |capability_server| {
            capability_server
                .all_peers()
                .into_iter()
                .take(max_peers as usize)
        })
        .await;

        Ok(Response::new(()))
    }

    async fn send_message_to_all(
        &self,
        request: tonic::Request<crate::grpc::sentry::OutboundMessageData>,
    ) -> Result<Response<()>, tonic::Status> {
        self.send_by_predicate(Some(request.into_inner()), |capability_server| {
            capability_server.all_peers()
        })
        .await;

        Ok(Response::new(()))
    }
}
