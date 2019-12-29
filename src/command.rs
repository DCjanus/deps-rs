use std::{path::PathBuf, time::Duration};

use structopt::StructOpt;
use url::Url;

lazy_static! {
    pub static ref COMMAND: Command = Command::from_args();
}

#[derive(Debug, StructOpt)]
pub struct Command {
    #[structopt(long)]
    pub proxy: Option<Url>,
    #[structopt(long, default_value = "./deps.cache")]
    pub cache: PathBuf,
    #[structopt(
        long,
        default_value = "https://github.com/rust-lang/crates.io-index.git"
    )]
    pub index: Url,
    #[structopt(long, default_value = "5m", parse(try_from_str = humantime::parse_duration))]
    pub interval: Duration,
}
