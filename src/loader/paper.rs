use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::info;
use serde::Deserialize;
use sha2::Sha256;

const BASE_URL: &str = "https://api.papermc.io/v2/projects/paper";

#[derive(Deserialize)]
struct Versions {
    versions: Vec<String>,
}

#[derive(Deserialize)]
struct Builds {
    builds: Vec<Build>,
}

#[derive(Clone, Default, Deserialize)]
struct Build {
    build: usize,
    downloads: Downloads,
}

#[derive(Clone, Default, Deserialize)]
struct Downloads {
    application: Application,
}

#[derive(Clone, Default, Deserialize)]
struct Application {
    sha256: String,
}

pub fn fetch(minecraft_version: &str, build: &str) -> Result<()> {
    let minecraft = if minecraft_version == "latest" {
        get_latest_version()?
    } else {
        minecraft_version.to_string()
    };

    let build = get_build(&minecraft, build)?;

    let formatted_url = format!(
        "{BASE_URL}/versions/{minecraft}/builds/{}/downloads/paper-{minecraft}-{}.jar",
        build.build, build.build,
    );

    let filename = format!("paper-{minecraft}-{}.jar", build.build);

    mup::download_with_checksum::<Sha256>(
        &formatted_url,
        &PathBuf::from(filename),
        &build.downloads.application.sha256,
    )?;

    Ok(())
}

fn get_latest_version() -> Result<String> {
    info!("fetching latest Minecraft version");

    let versions: Versions = mup::get_json(BASE_URL)?;

    let latest = versions
        .versions
        .last()
        .ok_or_else(|| anyhow!("could not get latest minecraft version"))?
        .to_string();

    Ok(latest.replace('"', ""))
}

fn get_build(minecraft_version: &str, build: &str) -> Result<Build> {
    let formatted_url = format!("{BASE_URL}/versions/{minecraft_version}/builds");

    info!("fetching build {build} for {minecraft_version}");

    let body: Builds = mup::get_json(&formatted_url)?;
    if build == "latest" {
        let first = body
            .builds
            .first()
            .ok_or_else(|| anyhow!("could not get latest loader version"))?;
        return Ok(first.clone());
    }

    let build_id: usize = build.parse()?;

    let latest_build = body
        .builds
        .iter()
        .find(|p| p.build == build_id)
        .ok_or_else(|| anyhow!("could not get specific loader version"))?;

    Ok(latest_build.clone())
}
