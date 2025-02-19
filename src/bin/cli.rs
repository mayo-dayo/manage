use anyhow::*;

use clap::Parser;

use manage::cli::Cli;
use manage::cli::Command;
use manage::command::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Cli { command } = Cli::parse();

    match command {
        Command::Ls => ls::ls().await,

        Command::Create => create::create().await,
    }
}
