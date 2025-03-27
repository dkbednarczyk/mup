#![allow(clippy::case_sensitive_file_extension_comparisons)]

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::Deserialize;

use crate::server::lockfile::Lockfile;

const BASE_URL: &str = "https://api.modrinth.com/v2";

#[derive(Clone, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub dependencies: Vec<ModrinthDependency>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<ProjectFile>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModrinthDependency {
    pub project_id: String,
    pub dependency_type: String,
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
    info!("Fetching project info for {id}");

    let formatted_url = format!("{BASE_URL}/project/{id}");
    let mut resp = ureq::get(formatted_url)
        .header("User-Agent", mup::USER_AGENT)
        .call()?;

    if resp.status() == 404 {
        return Err(anyhow!("project {id} does not exist"));
    }

    let project_info: ProjectInfo = resp.body_mut().read_json()?;

    if project_info.server_side == "unsupported" {
        return Err(anyhow!("project {id} does not support server-side"));
    }

    if project_info.server_side == "unknown" {
        warn!("project {id} may not support server-side");
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

    let dependencies = if version_info.dependencies.is_empty() {
        None
    } else {
        Some(
            version_info
                .dependencies
                .iter()
                .map(super::Dependency::from)
                .collect(),
        )
    };

    let info = super::Info {
        name: project_info.slug,
        id: project_info.id,
        version: version_info.id,
        source: String::from("modrinth"),
        download_url: project_file.url.clone(),
        checksum: Some(super::Checksum {
            method: String::from("sha512"),
            hash: project_file.hashes.sha512.clone(),
        }),
        dependencies,
    };

    Ok(info)
}

fn get_specific_version(
    slug: &str,
    version: &str,
    minecraft_version: &String,
    loader: &String,
) -> Result<Version> {
    info!("fetching version {version} of {slug}");

    let formatted_url = format!("{BASE_URL}/version/{version}");
    let mut resp = ureq::get(formatted_url)
        .header("User-Agent", mup::USER_AGENT)
        .call()?;

    if resp.status() == 404 {
        return Err(anyhow!("version {version} does not exist"));
    }

    let resp: Version = resp.body_mut().read_json()?;

    if slug != resp.project_id {
        return Err(anyhow!(
            "version id {version} is not a part of project {slug}",
        ));
    }

    if !resp.game_versions.contains(minecraft_version) {
        return Err(anyhow!(
            "version {version} does not support Minecraft {minecraft_version}"
        ));
    }

    if !resp.loaders.contains(loader) {
        return Err(anyhow!("version {version} does not support {loader}"));
    }

    Ok(resp)
}

fn get_latest_version(slug: &str, minecraft_version: &String, loader: &String) -> Result<Version> {
    info!("fetching latest version of {slug}");

    let formatted_url = format!("{BASE_URL}/project/{slug}/version");
    let mut resp = ureq::get(formatted_url)
        .header("User-Agent", mup::USER_AGENT)
        .query(
            "game_versions",
            format!("[\"{minecraft_version}\"]").as_str(),
        )
        .query("loaders", format!("[\"{loader}\"]").as_str())
        .call()?;

    if resp.status() == 404 {
        return Err(anyhow!("{slug} has no valid versions"));
    }

    let versions: Vec<Version> = resp.body_mut().read_json()?;

    let version = versions
        .iter()
        .find(|p| p.game_versions.contains(minecraft_version) && p.loaders.contains(loader))
        .ok_or_else(|| {
            anyhow!(
                "{slug} for {loader} has no version that supports Minecraft {minecraft_version}"
            )
        })?;

    Ok(version.clone())
}
