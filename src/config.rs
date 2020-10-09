use clap::Clap;
use derivative::Derivative;

#[derive(Derivative, Clap)]
#[clap(
    name = "ethereum-sentry",
    about = "Sentry for running on Ethereum P2P network"
)]
#[derivative(Debug)]
pub struct Opts {
    #[clap(long, env)]
    #[derivative(Debug = "ignore")]
    pub node_key: Option<String>,
    #[clap(long, env, default_value = "0.0.0.0:30303")]
    pub listen_addr: String,
    #[clap(long, env)]
    pub web3_addr: Option<String>,
    #[clap(long, env)]
    pub control_addr: Option<String>,
    #[clap(long, env, default_value = "all.mainnet.ethdisco.net")]
    pub dnsdisc_address: String,
}
