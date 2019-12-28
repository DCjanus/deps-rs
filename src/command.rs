use structopt::StructOpt;
use url::Url;

lazy_static! {
    pub static ref COMMAND: Command = Command::from_args();
}

#[derive(Debug, StructOpt)]
pub struct Command {
    #[structopt(long)]
    pub proxy: Option<Url>,
}
