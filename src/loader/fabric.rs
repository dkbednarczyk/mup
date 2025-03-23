use std::path::Path;

use anyhow::{anyhow, Result};
use log::info;
use serde::Deserialize;

const BASE_URL: &str = "https://meta.fabricmc.net/v2/versions";

#[derive(Clone, Deserialize)]
struct Version {
    version: String,
}

pub fn fetch(minecraft_version: &str, loader_version: &str) -> Result<()> {
    let game = get_version("/game", minecraft_version)?.version;
    let loader = get_version("/loader", loader_version)?.version;

    info!("fetching latest installer");

    let formatted_url = format!("{BASE_URL}/installer");
    let resp: Vec<Version> = mup::get_json(&formatted_url)?;

    let installer = &resp
        .first()
        .ok_or_else(|| anyhow!("failed to retrieve latest installer"))?
        .version;

    info!("downloading jarfile");

    let formatted_url = format!("{BASE_URL}/loader/{game}/{loader}/{installer}/server/jar");
    mup::download(&formatted_url, Path::new("fabric.jar"))?;

    Ok(())
}

fn get_version(path: &str, version: &str) -> Result<Version> {
    let stripped = path.strip_prefix('/').unwrap();

    info!("fetching information for {stripped} version {version}");

    let formatted_url = format!("{BASE_URL}{path}");
    let versions: Vec<Version> = mup::get_json(&formatted_url)?;

    if version == "latest" {
        return versions
            .first()
            .ok_or_else(|| anyhow!("failed to fetch requested minecraft version"))
            .cloned();
    }

    versions
        .iter()
        .find(|p| p.version == version)
        .ok_or_else(|| anyhow!("{stripped} version {version} does not exist"))
        .cloned()
}
