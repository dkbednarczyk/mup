use std::{fs::File, io};

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

    let formatted_url = format!("{BASE_URL}/installer");

    info!("fetching latest installer");

    let resp: Vec<Version> = ureq::get(&formatted_url)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json()?;

    let installer = &resp
        .first()
        .ok_or_else(|| anyhow!("failed to retrieve latest installer"))?
        .version;

    let formatted_url = format!("{BASE_URL}/loader/{game}/{loader}/{installer}/server/jar");

    info!("downloading jarfile");

    let mut resp = ureq::get(&formatted_url)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?;

    let mut body = resp.body_mut().as_reader();

    let mut file = File::create("fabric.jar")?;
    io::copy(&mut body, &mut file)?;

    Ok(())
}

fn get_version(path: &str, version: &str) -> Result<Version> {
    let stripped = path.strip_prefix('/').unwrap();

    info!("fetching information for {stripped} version {version}");

    let versions: Vec<Version> = ureq::get(&format!("{BASE_URL}{path}"))
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json()?;

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
