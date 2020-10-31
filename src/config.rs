use cidr::IpCidr;
use clap::Clap;
use devp2p::NodeRecord;
use educe::Educe;
use url::Url;

#[derive(Educe, Clap)]
#[clap(
    name = "ethereum-sentry",
    about = "Service that listens to Ethereum's P2P network, serves information to other nodes, and provides gRPC interface to clients to interact with the network."
)]
#[educe(Debug)]
pub struct Opts {
    #[clap(long, env)]
    #[educe(Debug(ignore))]
    pub node_key: Option<String>,
    #[clap(long, env, default_value = "30303")]
    pub listen_port: u16,
    #[clap(long, env)]
    pub cidr: Option<IpCidr>,
    #[clap(long, env, default_value = "0.0.0.0:8000")]
    pub sentry_addr: String,
    #[clap(long, env)]
    pub dnsdisc: bool,
    #[clap(long, env, default_value = "all.mainnet.ethdisco.net")]
    pub dnsdisc_address: String,
    #[clap(long, env)]
    pub discv4: bool,
    #[clap(long, env, default_value = "30303")]
    pub discv4_port: u16,
    #[clap(long, env)]
    pub discv4_bootnodes: Vec<discv4::NodeRecord>,
    #[clap(long, env)]
    pub discv5: bool,
    #[clap(long, env)]
    pub discv5_enr: Option<discv5::Enr>,
    #[clap(long, env, default_value = "0.0.0.0:30304")]
    pub discv5_addr: String,
    #[clap(long, env)]
    pub discv5_bootnodes: Vec<NodeRecord>,
    #[clap(long, env)]
    pub reserved_peers: Vec<NodeRecord>,
    #[clap(long, env, default_value = "50")]
    pub max_peers: usize,
    #[clap(long, env)]
    pub web3_addr: Option<Url>,
    #[clap(long, env)]
    pub control_addr: Option<Url>,
}
