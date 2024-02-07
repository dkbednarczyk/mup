use std::path::PathBuf;

use anyhow::anyhow;
use pap::download_with_checksum;
use serde::Deserialize;
use sha2::Sha256;

const BASE_URL: &str = "https://api.papermc.io/v2/projects/paper";

#[derive(Clone, Debug, Deserialize)]
struct Versions {
    versions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Builds {
    builds: Vec<Build>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Build {
    build: usize,
    downloads: Downloads,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Downloads {
    application: Application,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Application {
    sha256: String,
}

pub fn fetch(
    minecraft_input: &Option<String>,
    build_input: &Option<String>,
) -> Result<(), anyhow::Error> {
    let mut minecraft = minecraft_input.as_deref().unwrap().to_string();

    if minecraft == "latest" {
        minecraft = get_latest_version()?;
    }

    let build = match build_input.as_deref().unwrap() {
        "latest" => get_latest_build(&minecraft)?,
        b => get_specific_build(&minecraft, b.parse()?)?,
    };

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
    let body: Versions = ureq::get(BASE_URL)
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    let latest = body
        .versions
        .last()
        .ok_or_else(|| anyhow!("could not get latest minecraft version"))?;

    Ok(latest.clone())
}

fn get_latest_build(minecraft_version: &str) -> Result<Build, anyhow::Error> {
    let formatted_url = format!("{BASE_URL}/versions/{minecraft_version}/builds");

    let body: Builds = ureq::get(formatted_url.as_str())
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    let latest = body
        .builds
        .last()
        .ok_or_else(|| anyhow!("could not get latest loader version"))?;

    Ok(latest.clone())
}

fn get_specific_build(minecraft_version: &str, build: usize) -> Result<Build, anyhow::Error> {
    let formatted_url = format!("{BASE_URL}/versions/{minecraft_version}/builds");

    let body: Builds = ureq::get(formatted_url.as_str())
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    let latest = body
        .builds
        .iter()
        .find(|p| p.build == build)
        .ok_or_else(|| anyhow!("could not get specific loader version"))?;

    Ok(latest.clone())
}
