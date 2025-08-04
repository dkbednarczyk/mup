use std::path::Path;

use anyhow::{anyhow, Result};
use log::info;
use serde::Deserialize;
use sha1::Sha1;

const BASE_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

#[derive(Deserialize)]
struct VersionManifest {
    latest: Latest,
    versions: Vec<VanillaVersion>,
}

#[derive(Deserialize)]
struct Latest {
    release: String,
    snapshot: String,
}

#[derive(Deserialize)]
struct VanillaVersion {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String,
}

#[derive(Deserialize)]
struct VersionData {
    downloads: Downloads,
}

#[derive(Deserialize)]
struct Downloads {
    server: DownloadInfo,
}

#[derive(Deserialize)]
struct DownloadInfo {
    url: String,
    sha1: String,
}

pub fn fetch(minecraft_version: &str, snapshot: bool) -> Result<()> {
    let version = get_version(minecraft_version, snapshot)?;

    if version.version_type == "snapshot" && !snapshot {
        return Err(anyhow!("--snapshot flag is required for snapshot versions"));
    }

    let version_data: VersionData = mup::get_json(&version.url)?;
    let filename = format!("vanilla-{}-server.jar", version.id);

    info!(
        "downloading jarfile to {} from {}",
        version.id, version_data.downloads.server.url
    );

    mup::download_with_checksum::<Sha1>(
        &version_data.downloads.server.url,
        Path::new(&filename),
        &version_data.downloads.server.sha1,
    )?;

    Ok(())
}

fn get_version(minecraft_version: &str, snapshot: bool) -> Result<VanillaVersion> {
    let manifest: VersionManifest = mup::get_json(BASE_URL)?;

    let version_id = if minecraft_version == "latest" {
        if snapshot {
            manifest.latest.snapshot
        } else {
            manifest.latest.release
        }
    } else {
        minecraft_version.to_string()
    };

    let version = manifest
        .versions
        .into_iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| anyhow!("Minecraft version {} not found", version_id))?;

    Ok(version)
}
