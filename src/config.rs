use clap::Clap;
use educe::Educe;
use url::Url;

#[derive(Educe, Clap)]
#[clap(
    name = "ethereum-sentry",
    about = "Sentry for running on Ethereum P2P network"
)]
#[educe(Debug)]
pub struct Opts {
    #[clap(long, env)]
    #[educe(Debug(ignore))]
    pub node_key: Option<String>,
    #[clap(long, env, default_value = "0.0.0.0:30303")]
    pub listen_addr: String,
    #[clap(long, env)]
    pub dnsdisc: bool,
    #[clap(long, env, default_value = "all.mainnet.ethdisco.net")]
    pub dnsdisc_address: String,
    #[clap(long, env)]
    pub discv5: bool,
    #[clap(long, env)]
    pub discv5_enr: Option<discv5::Enr>,
    #[clap(long, env, default_value = "0.0.0.0:30303")]
    pub discv5_addr: String,
    #[clap(long, env)]
    pub discv5_bootnodes: Vec<devp2p::NodeRecord>,
    #[clap(long, env)]
    pub reserved_peers: Vec<devp2p::NodeRecord>,
    #[clap(long, env, default_value = "50")]
    pub max_peers: usize,
    #[clap(long, env)]
    pub web3_addr: Option<Url>,
    #[clap(long, env)]
    pub control_addr: Option<Url>,
}
