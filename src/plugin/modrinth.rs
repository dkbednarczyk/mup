#![allow(clippy::case_sensitive_file_extension_comparisons)]

use anyhow::{anyhow, Result};
use log::info;
use mup::FAKE_USER_AGENT;
use serde::Deserialize;

use crate::server::lockfile::Lockfile;

const BASE_URL: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub dependencies: Vec<super::Dependency>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<ProjectFile>,
}

#[derive(Clone, Deserialize)]
pub struct ProjectFile {
    pub hashes: Hashes,
    pub url: String,
    filename: String,
}

#[derive(Clone, Deserialize)]
pub struct Hashes {
    pub sha512: String,
}

#[derive(Deserialize)]
pub struct ProjectInfo {
    pub slug: String,
    server_side: String,
    id: String,
    loaders: Vec<String>,
    game_versions: Vec<String>,
    versions: Vec<String>,
}

pub fn fetch(lockfile: &Lockfile, id: &str, version: &str) -> Result<super::Info> {
    let formatted_url = format!("{BASE_URL}/project/{id}");

    info!("Fetching project info for {id}");

    let project_info: ProjectInfo = ureq::get(&formatted_url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json()?;

    if project_info.server_side == "unsupported" {
        return Err(anyhow!("client side"));
    }

    if !project_info.loaders.contains(&lockfile.loader.name) {
        return Err(anyhow!(
            "project {id} does not support {}",
            lockfile.loader.name
        ));
    }

    if !project_info
        .game_versions
        .contains(&lockfile.loader.minecraft_version)
    {
        return Err(anyhow!(
            "project does not support Minecraft version {}",
            lockfile.loader.minecraft_version
        ));
    }

    if version != "latest" && !project_info.versions.contains(&version.to_string()) {
        return Err(anyhow!("project version {version} does not exist"));
    }

    if !project_info
        .game_versions
        .contains(&lockfile.loader.minecraft_version)
    {
        return Err(anyhow!(
            "project does not support minecraft version {}",
            lockfile.loader.minecraft_version
        ));
    }

    let version_info = if version == "latest" {
        get_latest_version(
            &project_info.slug,
            &lockfile.loader.minecraft_version,
            &lockfile.loader.name,
        )?
    } else {
        get_specific_version(
            &project_info.slug,
            version,
            &lockfile.loader.minecraft_version,
            &lockfile.loader.name,
        )?
    };

    let project_file = version_info
        .files
        .iter()
        .find(|f| f.filename.ends_with(".jar"))
        .unwrap();

    let info = super::Info {
        slug: project_info.slug,
        id: project_info.id,
        version: version_info.id,
        source: format!("modrinth#{}", project_file.url),
        checksum: Some(format!("sha512#{}", project_file.hashes.sha512)),
        dependencies: version_info
            .dependencies
            .iter()
            .map(|d| super::Dependency {
                id: d.id.clone(),
                required: d.required,
            })
            .collect(),
    };

    Ok(info)
}

fn get_specific_version(
    slug: &str,
    version: &str,
    minecraft_version: &String,
    loader: &String,
) -> Result<Version> {
    let formatted_url = format!("{BASE_URL}/version/{version}");

    info!("fetching version {version} of {slug}");

    let resp: Version = ureq::get(&formatted_url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json()?;

    if slug != resp.project_id {
        return Err(anyhow!(
            "version id {version} is not a part of project {}",
            slug
        ));
    }

    if !resp.game_versions.contains(minecraft_version) {
        return Err(anyhow!(
            "version id {version} does not support Minecraft version {minecraft_version}"
        ));
    }

    if !resp.loaders.contains(loader) {
        return Err(anyhow!(
            "project version {version} does not support loader {loader}",
        ));
    }

    Ok(resp)
}

fn get_latest_version(slug: &str, minecraft_version: &String, loader: &String) -> Result<Version> {
    let formatted_url = format!("{BASE_URL}/project/{slug}/version");

    let mut req = ureq::get(&formatted_url)
        .header("User-Agent", FAKE_USER_AGENT)
        .query(
            "game_versions",
            format!("[\"{minecraft_version}\"]").as_str(),
        );

    if !loader.is_empty() {
        req = req.query("loaders", format!("[\"{loader}\"]").as_str());
    }

    info!("fetching latest version of {slug}");

    let resp: Vec<Version> = req.call()?.body_mut().read_json()?;

    let version = resp
        .iter()
        .find(|p| p.game_versions.contains(minecraft_version))
        .ok_or_else(|| anyhow!("could not find a matching version"))?;

    if !version.loaders.contains(loader) {
        return Err(anyhow!(
            "project version ID {} does not support loader {loader}",
            version.id
        ));
    }

    Ok(version.clone())
}
