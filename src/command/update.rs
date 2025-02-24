use crate::mayo::Mayo;
use crate::versioning;

use anyhow::*;

use inquire::Select;

pub async fn update() -> Result<()> {
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

    let latest_version = versioning::get_latest_compatible_app_version()
        //
        .await
        //
        .context("failed to get the latest app version")?;

    servers.retain(|server| server.parameters.version < latest_version);

    servers.sort_unstable();

    if servers.is_empty() {
        println!("All servers are up-to-date 🙂");

        return Ok(());
    }

    let Some(server) = Select::new(
        //
        "Select the server:",
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

    mayo.update_server(
        //
        server,
        //
        latest_version,
    )
    //
    .await
    //
    .context("failed to update the server")?;

    Ok(())
}
