const NEXUSMODS_API_BASE_URL: &str = "https://api.nexusmods.com/v1";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CachedModInfo {
    name: String,
    author: String,
}

impl CachedModInfo {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn author(&self) -> &str {
        &self.author
    }
}

#[derive(Debug, serde::Deserialize)]
struct NexusModsModResponse {
    name: String,
    author: String,
}

pub async fn fetch_mod_info(
    api_key: &str,
    game_domain_name: &str,
    mod_id: u32,
) -> Result<CachedModInfo, reqwest::Error> {
    let url = format!("{NEXUSMODS_API_BASE_URL}/games/{game_domain_name}/mods/{mod_id}.json");
    let response = reqwest::Client::new()
        .get(url)
        .header("apikey", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?
        .error_for_status()?
        .json::<NexusModsModResponse>()
        .await?;

    Ok(CachedModInfo { name: response.name, author: response.author })
}
