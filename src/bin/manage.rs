use manage::app::App;

use anyhow::Result;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    human_panic::setup_panic!(
        //
        human_panic::Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            //
            .homepage("https://github.com/mayo-dayo/manage")
    );

    let app = App::try_init()?;

    app
        //
        .start()
        //
        .await?;

    Ok(())
}
