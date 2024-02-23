use std::path::PathBuf;

use anyhow::anyhow;
use log::info;
use pap::download_with_checksum;
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

pub fn fetch(minecraft_version: &str, build: &str) -> Result<(), anyhow::Error> {
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

    download_with_checksum::<Sha256>(
        &formatted_url,
        &PathBuf::from(filename),
        &build.downloads.application.sha256,
    )?;

    Ok(())
}

fn get_latest_version() -> Result<String, anyhow::Error> {
    info!("fetching latest Minecraft version");

    let body: Versions = ureq::get(BASE_URL)
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    let latest = body
        .versions
        .last()
        .ok_or_else(|| anyhow!("could not get latest minecraft version"))?
        .to_string();

    Ok(latest.replace('"', ""))
}

fn get_build(minecraft_version: &str, build: &str) -> Result<Build, anyhow::Error> {
    let formatted_url = format!("{BASE_URL}/versions/{minecraft_version}/builds");

    info!("fetching build {build} for {minecraft_version}");

    let body: Builds = ureq::get(formatted_url.as_str())
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    if build == "latest" {
        return Ok(body.builds.first().unwrap().clone());
    }

    let build_id: usize = build.parse()?;

    let latest_build = body
        .builds
        .iter()
        .find(|p| p.build == build_id)
        .ok_or_else(|| anyhow!("could not get specific loader version"))?;

    Ok(latest_build.clone())
}
