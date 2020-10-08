#![allow(dead_code)]

use crate::{
    config::*,
    eth::*,
    grpc::control::{InboundMessage, StatusData},
    services::*,
};
use anyhow::anyhow;
use arrayvec::ArrayString;
use async_trait::async_trait;
use clap::Clap;
use devp2p::*;
use hex_literal::hex;
use k256::ecdsa::SigningKey;
use num_traits::{FromPrimitive, ToPrimitive};
use rand::rngs::OsRng;
use rlp::Rlp;
use std::{
    convert::{TryFrom, TryInto},
    sync::Arc,
};
use tracing::*;
use tracing_subscriber::EnvFilter;
use trust_dns_resolver::{config::*, TokioAsyncResolver};

mod config;
mod eth;
mod grpc;
mod services;
mod types;

#[derive(Debug)]
struct DummyControl;

#[async_trait]
impl Control for DummyControl {
    async fn forward_inbound_message(&self, message: InboundMessage) -> anyhow::Result<()> {
        debug!("Received inbound message: {:?}", message);
        Ok(())
    }
    async fn get_status(&self) -> anyhow::Result<StatusData> {
        Ok(StatusData {
            network_id: 1,
            total_difficulty: 17608636743620256866935_u128.to_be_bytes().to_vec(),
            best_hash: hex!("28042e7e4d35a3482bf5f0d862501868b04c1734f483ceae3bf1393561951829")
                .to_vec(),
            genesis_hash: hex!("d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3")
                .to_vec(),
        })
    }
}

#[derive(Debug)]
struct CapabilityServerImpl<C, DP> {
    control: C,
    data_provider: DP,
}

#[async_trait]
impl<C: Control, DP: DataProvider> CapabilityServer for CapabilityServerImpl<C, DP> {
    async fn on_peer_connect(&self, _: PeerId) -> PeerConnectOutcome {
        if let Ok(status_data) = self.control.get_status().await {
            if let Ok(status_message) = StatusMessage::try_from(status_data) {
                return PeerConnectOutcome::Retain {
                    hello: Some(Message {
                        id: 0,
                        data: rlp::encode(&status_message).into(),
                    }),
                };
            }
        }

        PeerConnectOutcome::Disavow
    }
    #[instrument(skip(peer, message), level = "debug", name = "ingress", fields(peer=&*peer.id.to_string(), id=message.id))]
    async fn on_ingress_message(
        &self,
        peer: IngressPeer,
        message: Message,
    ) -> Result<(Option<Message>, Option<ReputationReport>), HandleError> {
        let Message { id, data } = message;

        debug!("Received message");

        match MessageId::from_usize(id).ok_or_else(|| anyhow!("unknown message"))? {
            MessageId::Status => match rlp::decode::<StatusMessage>(&data) {
                Ok(v) => {
                    info!("Decoded status message: {:?}", v);
                    Ok((None, None))
                }
                Err(e) => {
                    info!("Failed to decode status message: {}! Kicking peer.", e);
                    Ok((None, Some(DisconnectReason::ProtocolBreach.into())))
                }
            },
            MessageId::GetBlockHeaders => {
                let selector = rlp::decode::<GetBlockHeaders>(&*data)?;
                info!("Block headers requested: {:?}", selector);

                let selector = if selector.max_headers > 1 {
                    // Just one. Fast case.
                    vec![selector.block]
                } else {
                    async {
                        let anchor = match selector.block {
                            BlockId::Number(num) => num,
                            BlockId::Hash(hash) => {
                                match self.data_provider.resolve_block_height(hash).await {
                                    Ok(Some(height)) => height,
                                    Ok(None) => {
                                        // this block does not exist, exit early.
                                        return vec![];
                                    }
                                    Err(e) => {
                                        warn!("Failed to resolve block {}: {}. Will query this one hash only", hash, e);
                                        return vec![BlockId::Hash(hash)];
                                    }
                                }
                            }
                        };

                        if selector.skip == 0 {
                            return vec![BlockId::Number(anchor)];
                        }

                        std::iter::once(anchor)
                            .chain((0..selector.max_headers).map(|i| {
                                if selector.reverse {
                                    anchor - selector.skip * i
                                } else {
                                    anchor + selector.skip * i
                                }
                            }))
                            .map(BlockId::Number)
                            .collect()
                    }
                    .await
                };

                let output = self
                    .data_provider
                    .get_block_headers(selector)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Failed to fetch block headers: {}", e);
                        vec![]
                    });

                Ok((
                    Some(Message {
                        id: MessageId::BlockHeaders.to_usize().unwrap(),
                        data: rlp::encode_list(&output).into(),
                    }),
                    None,
                ))
            }
            MessageId::GetBlockBodies => {
                let block = Rlp::new(&*data).as_list()?;
                info!("Block bodies requested: {:?}", block);

                let output = self
                    .data_provider
                    .get_block_bodies(block)
                    .await
                    .unwrap_or_else(|e| {
                        warn!("Failed to fetch block bodies: {}", e);
                        vec![]
                    });

                Ok((
                    Some(Message {
                        id: MessageId::BlockBodies.to_usize().unwrap(),
                        data: rlp::encode_list(&output).into(),
                    }),
                    None,
                ))
            }
            _ => Ok((None, None)),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("ethereum_sentry=info".parse().unwrap()),
        )
        .init();

    let opts = Opts::parse();

    let secret_key = if let Some(data) = opts.node_key {
        SigningKey::new(&hex::decode(data)?)?
    } else {
        SigningKey::random(&mut OsRng)
    };

    info!("Starting Ethereum sentry");

    info!(
        "Node ID: {}",
        hex::encode(devp2p::util::pk2id(&secret_key.verify_key()).as_bytes())
    );

    let dns_resolver = dnsdisc::Resolver::new(Arc::new(
        TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).await?,
    ));

    let discovery = DnsDiscovery::new(Arc::new(dns_resolver), opts.dnsdisc_address, None);

    let client = RLPxNodeBuilder::new()
        .with_listen_options(ListenOptions {
            discovery: Some(DiscoveryOptions {
                discovery: Arc::new(tokio::sync::Mutex::new(discovery)),
                tasks: 1_usize.try_into().unwrap(),
            }),
            max_peers: 50,
            addr: opts.listen_addr.parse()?,
        })
        .with_client_version(format!("sentry/v{}", env!("CARGO_PKG_VERSION")))
        .build(secret_key)
        .await?;

    info!("RLPx node listening at {}", opts.listen_addr);

    let _handle = client.register(
        CapabilityInfo {
            name: CapabilityName(ArrayString::from("eth").unwrap()),
            version: 63,
            length: 17,
        },
        Arc::new(CapabilityServerImpl {
            control: DummyControl,
            data_provider: DummyDataProvider,
        }),
    );

    futures::future::pending().await
}
