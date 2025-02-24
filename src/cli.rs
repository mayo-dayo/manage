use clap::Args;
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

    /// Update a server
    Update,

    /// Manage server invites
    Invite(Invite),
}

#[derive(Args)]
pub struct Invite {
    #[command(subcommand)]
    pub command: InviteCommand,
}

#[derive(Subcommand)]
pub enum InviteCommand {
    /// Create an invite
    Create,
}
