use crate::mayo::Mayo;
use crate::parameters::Parameters;

use anyhow::*;

pub async fn create() -> Result<()> {
    let mayo = Mayo::try_new()?;

    let Some(parameters) = Parameters::inquire().await? else {
        return Ok(());
    };

    let container_id = mayo
        //
        .create_server(parameters)
        //
        .await
        //
        .context("failed to create a server")?;

    println!("{container_id}");

    Ok(())
}
