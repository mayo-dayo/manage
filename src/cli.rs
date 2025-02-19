use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create and run a new server
    Create,

    /// List servers
    Ls,
}
