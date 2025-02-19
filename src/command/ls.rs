use crate::mayo::Mayo;
use crate::server::Server;
use crate::parameters::Parameters;

use anyhow::*;

use comfy_table::presets;
use comfy_table::*;

pub async fn ls() -> Result<()> {
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
    } else {
        let mut table = Table::new();

        table
            //
            .load_preset(presets::UTF8_FULL)
            //
            .set_content_arrangement(ContentArrangement::Dynamic)
            //
            .set_header(vec![
                //
                "name",
                //
                "version",
                //
                "state",
                //
                "port",
                //
                "authentication",
                //
                "tls",
            ]);

        servers.sort_unstable();

        for server in servers {
            let Server {
                state,

                parameters:
                    Parameters {
                        name,

                        version,

                        port,

                        authentication,

                        tls,
                    },
                ..
            } = server;

            table.add_row(vec![
                //
                Cell::new(name),
                //
                Cell::new(version),
                //
                Cell::new(state),
                //
                Cell::new(port),
                //
                Cell::new(authentication),
                //
                Cell::new(tls),
            ]);
        }

        println!("{table}");
    }

    Ok(())
}
