use std::{path::PathBuf, time::Duration};

use once_cell::sync::Lazy;
use sled::Db;
use structopt::StructOpt;
use url::Url;

use crate::utils::AnyResult;

static COMMAND: Lazy<Command> = Lazy::new(Command::from_args);
static DATABASE: Lazy<Db> = Lazy::new(|| sled::open(&db_path()).unwrap());

#[derive(Debug, StructOpt)]
struct Command {
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

pub fn init() -> AnyResult {
    Lazy::force(&COMMAND);
    Lazy::force(&DATABASE);

    Ok(())
}

pub fn proxy() -> Option<&'static str> {
    COMMAND.proxy.as_ref().map(|x| x.as_str())
}

pub fn db_path() -> PathBuf {
    COMMAND.cache.join("database")
}

pub fn tick_interval() -> Duration {
    COMMAND.interval
}

pub fn cache_dir() -> PathBuf {
    COMMAND.cache.join("crates.io-index")
}

pub fn index_url() -> &'static str {
    COMMAND.index.as_str()
}

pub fn database() -> &'static Db {
    &DATABASE
}
