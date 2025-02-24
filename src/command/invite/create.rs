use crate::invite::Invite;
use crate::mayo::Mayo;

use anyhow::*;

use inquire::Select;

pub async fn create() -> Result<()> {
    let mayo = Mayo::try_new()?;

    let mut servers = mayo
        //
        .list_servers()
        //
        .await
        //
        .context("failed to list servers")?;

    if servers.is_empty() {
        println!("No servers 😔");

        return Ok(());
    }

    servers.sort_unstable();

    let Some(server) = Select::new(
        //
        "Please select the server:",
        //
        servers,
    )
    //
    .prompt_skippable()
    //
    .context("failed to inquire the server")?
    //
    else {
        return Ok(());
    };

    let Some(invite) = Invite::inquire()? else {
        return Ok(());
    };

    mayo.create_invite(server, invite)
        //
        .await
        //
        .context("failed to create an invite")?;

    Ok(())
}
