#![allow(dead_code)]

use crate::{config::*, eth::*, grpc::control::InboundMessage, services::*};
use anyhow::{anyhow, bail, Context};
use arrayvec::ArrayString;
use async_trait::async_trait;
use clap::Clap;
use devp2p::*;
use enr::CombinedKey;
use futures::stream::StreamExt;
use k256::ecdsa::SigningKey;
use num_traits::{FromPrimitive, ToPrimitive};
use parking_lot::RwLock;
use rand::rngs::OsRng;
use rlp::Rlp;
use std::{collections::HashMap, sync::Arc, time::Duration};
use task_group::TaskGroup;
use tokio::sync::Mutex as AsyncMutex;
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
    async fn get_status_data(&self) -> anyhow::Result<StatusData> {
        bail!("Not implemented")
    }
}

#[derive(Debug)]
struct CapabilityServerImpl<C, DP> {
    status_message: Arc<RwLock<Option<StatusMessage>>>,
    control: C,
    data_provider: DP,
}

#[async_trait]
impl<C: Control, DP: DataProvider> CapabilityServer for CapabilityServerImpl<C, DP> {
    async fn on_peer_connect(&self, _: PeerId) -> PeerConnectOutcome {
        if let Some(status_message) = self.status_message.read().clone() {
            return PeerConnectOutcome::Retain {
                hello: Some(Message {
                    id: 0,
                    data: rlp::encode(&status_message).into(),
                }),
            };
        }

        PeerConnectOutcome::Disavow
    }
    #[instrument(skip(self, peer, message), level = "debug", fields(peer=&*peer.id.to_string(), id=message.id))]
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
                    .filter_map(|res| async move {
                        match res {
                            Err(e) => {
                                warn!("{}", e);
                                None
                            }
                            Ok(v) => Some(v),
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;

                let id = MessageId::BlockHeaders;
                let data = rlp::encode_list(&output);

                info!("Replying: {:?} / {}", id, hex::encode(&data));

                Ok((
                    Some(Message {
                        id: id.to_usize().unwrap(),
                        data: data.into(),
                    }),
                    None,
                ))
            }
            MessageId::GetBlockBodies => {
                let blocks = Rlp::new(&*data).as_list()?;
                info!("Block bodies requested: {:?}", blocks);

                let output: Vec<_> = self
                    .data_provider
                    .get_block_bodies(blocks)
                    .filter_map(|res| async move {
                        match res {
                            Err(e) => {
                                warn!("{}", e);
                                None
                            }
                            Ok(v) => Some(v),
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;

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
        .with_env_filter(EnvFilter::from_default_env())
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

    let mut discovery_tasks: Vec<Arc<AsyncMutex<dyn Discovery>>> = vec![];

    if opts.dnsdisc {
        info!("Starting DNS discovery fetch from {}", opts.dnsdisc_address);
        let dns_resolver = dnsdisc::Resolver::new(Arc::new(
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).await?,
        ));

        discovery_tasks.push(Arc::new(AsyncMutex::new(DnsDiscovery::new(
            Arc::new(dns_resolver),
            opts.dnsdisc_address,
            None,
        ))));
    }

    if opts.discv5 {
        let mut svc = discv5::Discv5::new(
            opts.discv5_enr
                .ok_or_else(|| anyhow!("discv5 ENR not specified"))?,
            CombinedKey::Secp256k1(SigningKey::new(secret_key.to_bytes().as_slice()).unwrap()),
            Default::default(),
        )
        .map_err(|e| anyhow!("{}", e))?;
        svc.start(opts.discv5_addr.parse()?)
            .map_err(|e| anyhow!("{}", e))
            .context("Failed to start discv5")?;
        info!("Starting discv5 at {}", opts.discv5_addr);
        discovery_tasks.push(Arc::new(AsyncMutex::new(svc)));
    }

    if !opts.reserved_peers.is_empty() {
        info!("Enabling reserved peers: {:?}", opts.reserved_peers);
        discovery_tasks.push(Arc::new(AsyncMutex::new(
            opts.reserved_peers
                .iter()
                .map(|&NodeRecord { addr, id }| (addr, id))
                .collect::<HashMap<_, _>>(),
        )))
    }

    let tasks = Arc::new(TaskGroup::new());

    let client = RLPxNodeBuilder::new()
        .with_task_group(tasks.clone())
        .with_listen_options(ListenOptions {
            discovery_tasks,
            max_peers: opts.max_peers,
            addr: opts.listen_addr.parse()?,
        })
        .with_client_version(format!("sentry/v{}", env!("CARGO_PKG_VERSION")))
        .build(secret_key)
        .await?;

    let data_provider: Arc<dyn DataProvider> = if let Some(addr) = opts.web3_addr {
        if addr.scheme() != "http" && addr.scheme() != "https" {
            bail!(
                "Invalid web3 data provider URL: {}. Should start with http:// or https://.",
                addr
            );
        }
        let addr = addr.to_string();
        info!("Using web3 data provider at {}", addr);
        Arc::new(Web3DataProvider::new(addr).context("Failed to start web3 data provider")?)
    } else {
        Arc::new(DummyDataProvider)
    };

    let control: Arc<dyn Control> = Arc::new(DummyControl);

    info!("RLPx node listening at {}", opts.listen_addr);

    let status_message: Arc<RwLock<Option<StatusMessage>>> = Default::default();

    tasks.spawn_with_name(
        {
            let status_message = status_message.clone();
            let control = control.clone();
            let data_provider = data_provider.clone();
            async move {
                loop {
                    let mut s = None;
                    match control.get_status_data().await {
                        Err(e) => {
                            debug!(
                                "Failed to get status from control, trying from data provider: {}",
                                e
                            );
                            match data_provider.get_status_data().await {
                                Err(e) => {
                                    debug!("Failed to fetch status from data provider: {}", e);
                                }
                                Ok(v) => {
                                    s = Some(v);
                                }
                            }
                        }
                        Ok(v) => {
                            s = Some(v);
                        }
                    }

                    if let Some(s) = &s {
                        debug!("Setting status data to {:?}", s);
                    } else {
                        warn!("Failed to fetch status data, server will not accept new peers.");
                    }
                    *status_message.write() = s.map(From::from);

                    tokio::time::delay_for(Duration::from_secs(5)).await;
                }
            }
        },
        "Status updater".into(),
    );

    let _handle = client.register(
        CapabilityInfo {
            name: CapabilityName(ArrayString::from("eth").unwrap()),
            version: 63,
            length: 17,
        },
        Arc::new(CapabilityServerImpl {
            status_message,
            control,
            data_provider,
        }),
    );

    loop {
        info!(
            "Current peers: {}/{}.",
            client.connected_peers().len(),
            opts.max_peers
        );

        tokio::time::delay_for(Duration::from_secs(5)).await;
    }
}
