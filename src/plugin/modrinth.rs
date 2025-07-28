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

#[derive(Clone, Deserialize)]
pub struct ModrinthDependency {
    #[serde(skip)]
    pub slug: String,
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
    let mut resp = mup::get(&formatted_url).call()?;

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

    let mut version_info = if version == "latest" {
        get_latest_version(lockfile, &project_info.slug)?
    } else {
        get_specific_version(lockfile, &project_info.slug, version)?
    };

    let project_file = version_info
        .files
        .iter()
        .find(|f| f.filename.ends_with(".jar"))
        .unwrap();

    let dependencies = if version_info.dependencies.is_empty() {
        None
    } else {
        for dep in &mut version_info.dependencies {
            if dep.project_id == project_info.id {
                return Err(anyhow!("project {id} depends on itself"));
            }

            dep.slug = get_project_name(&dep.project_id)?;
        }

        let deps = version_info
            .dependencies
            .into_iter()
            .map(super::Dependency::from)
            .collect();

        Some(deps)
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

fn get_project_name(project_id: &str) -> Result<String> {
    info!("fetching project name for project id {project_id}");

    let formatted_url = format!("{BASE_URL}/project/{project_id}");
    let mut resp = mup::get(&formatted_url).call()?;

    if resp.status() == 404 {
        return Err(anyhow!("project {project_id} does not exist"));
    }

    let resp: ProjectInfo = resp.body_mut().read_json()?;

    Ok(resp.slug)
}

fn get_specific_version(lockfile: &Lockfile, slug: &str, version: &str) -> Result<Version> {
    info!("fetching version {version} of {slug}");

    let formatted_url = format!("{BASE_URL}/version/{version}");
    let mut resp = mup::get(&formatted_url).call()?;

    if resp.status() == 404 {
        return Err(anyhow!("version {version} does not exist"));
    }

    let resp: Version = resp.body_mut().read_json()?;

    if slug != resp.project_id {
        return Err(anyhow!(
            "version id {version} is not a part of project {slug}",
        ));
    }

    if !resp
        .game_versions
        .contains(&lockfile.loader.minecraft_version)
    {
        return Err(anyhow!(
            "version {version} does not support Minecraft {}",
            lockfile.loader.minecraft_version
        ));
    }

    if !resp.loaders.contains(&lockfile.loader.name) {
        return Err(anyhow!(
            "version {version} does not support {}",
            lockfile.loader.name
        ));
    }

    Ok(resp)
}

fn get_latest_version(lockfile: &Lockfile, slug: &str) -> Result<Version> {
    info!("fetching latest version of {slug}");

    let loader = &lockfile.loader.name;
    let version = &lockfile.loader.minecraft_version;

    let formatted_url = format!("{BASE_URL}/project/{slug}/version");
    let mut resp = mup::get(&formatted_url)
        .query("game_versions", format!("[\"{version}\"]").as_str())
        .query("loaders", format!("[\"{loader}\"]").as_str())
        .call()?;

    if resp.status() == 404 {
        return Err(anyhow!("{slug} has no valid versions"));
    }

    let versions: Vec<Version> = resp.body_mut().read_json()?;

    let version = versions
        .into_iter()
        .find(|p| p.game_versions.contains(version) && p.loaders.contains(loader))
        .ok_or_else(|| {
            anyhow!("{slug} for {loader} has no version that supports Minecraft {version}")
        })?;

    Ok(version)
}
