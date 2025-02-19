use anyhow::*;

pub const REGISTRY: &str = "ghcr.io";

pub const USERNAME: &str = "mayo-dayo";

pub const REPOSITORY: &str = "app";

async fn get_token() -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Body {
        token: String,
    }

    let url = format!(
        //
        "https://{}/token?scope=repository:{}/{}:pull",
        //
        REGISTRY,
        //
        USERNAME,
        //
        REPOSITORY
    );

    let Body { token } = reqwest::get(url)
        //
        .await
        //
        .context("failed to send http request")?
        //
        .json::<Body>()
        //
        .await
        //
        .context("failed to receive http response")?;

    Ok(token)
}

pub async fn get_app_tags() -> Result<Vec<String>> {
    let token = get_token()
        //
        .await
        //
        .context("failed to get registry token")?;

    #[derive(serde::Deserialize)]
    struct Body {
        tags: Vec<String>,
    }

    let url = format!(
        //
        "https://{}/v2/{}/{}/tags/list",
        //
        REGISTRY,
        //
        USERNAME,
        //
        REPOSITORY
    );

    let Body { tags } = reqwest::Client::new()
        //
        .get(url)
        //
        .bearer_auth(token)
        //
        .send()
        //
        .await
        //
        .context("failed to send http request")?
        //
        .json::<Body>()
        //
        .await
        //
        .context("failed to receive http response")?;

    Ok(tags)
}
