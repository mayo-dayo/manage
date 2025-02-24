use anyhow::*;

use clap::Parser;

use manage::cli::*;
use manage::command::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let Cli { command } = Cli::parse();

    match command {
        Command::Ls => ls::ls().await,

        Command::Create => create::create().await,

        Command::Update => update::update().await,

        Command::Invite(Invite { command }) => match command {
            InviteCommand::Create => invite::create::create().await,
        },
    }
}
