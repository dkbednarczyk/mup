use std::collections::HashMap;

use crate::server::lockfile::Lockfile;

use anyhow::{anyhow, Result};
use log::info;
use serde::Deserialize;
use versions::SemVer;

const BASE_URL: &str = "https://hangar.papermc.io/api/v1";

#[derive(Deserialize)]
struct VersionInfo {
    downloads: HashMap<String, Download>,
    #[serde(rename = "pluginDependencies")]
    dependencies: HashMap<String, Vec<super::Dependency>>,
    #[serde(rename = "platformDependencies")]
    platform_dependencies: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct Download {
    #[serde(rename = "fileInfo")]
    file_info: FileInfo,
    #[serde(rename = "downloadUrl")]
    url: String,
}

#[derive(Deserialize)]
struct FileInfo {
    #[serde(rename = "sha256Hash")]
    sha256: String,
}

#[derive(Deserialize)]
struct ProjectInfo {
    name: String,
}

pub fn fetch(lockfile: &Lockfile, project_id: &str, version: &str) -> Result<super::Info> {
    info!("fetching info of project {project_id}");

    let formatted_url = format!("{BASE_URL}/projects/{project_id}");
    let mut resp = ureq::get(formatted_url)
        .header("User-Agent", mup::USER_AGENT)
        .call()?;

    if resp.status() == 404 {
        return Err(anyhow!("project {project_id} does not exist"));
    }

    let project_info: ProjectInfo = resp.body_mut().read_json()?;
    let project = project_info.name;

    let version = if version == "latest" {
        info!("fetching latest version of project {project}");

        let formatted_url = format!("{BASE_URL}/projects/{project}/latestrelease");

        mup::get_string(&formatted_url)?
    } else {
        version.into()
    };

    info!("fetching info for {project} v{version}");

    let formatted_url = format!("{BASE_URL}/projects/{project}/versions/{version}");
    let version_info: VersionInfo = mup::get_json(&formatted_url)?;

    let loader = lockfile.loader.name.to_uppercase();
    if !version_info.platform_dependencies.contains_key(&loader) {
        return Err(anyhow!(
            "{project} version {version} does not support {loader}"
        ));
    }

    let minecraft_version = SemVer::new(&lockfile.loader.minecraft_version).unwrap();
    let is_compatible = version_info.platform_dependencies[&loader]
        .iter()
        .filter_map(SemVer::new)
        .any(|v| v == minecraft_version);

    if !is_compatible {
        return Err(anyhow!("{project} version {version} is incompatible with Minecraft version {minecraft_version}"));
    }

    let dependencies = if version_info.dependencies.contains_key(&loader) {
        Some(version_info.dependencies[&loader].clone())
    } else {
        None
    };

    let info = super::Info {
        name: project.clone(),
        id: project,
        version,
        source: String::from("hangar"),
        download_url: version_info.downloads[&loader].url.clone(),
        checksum: Some(super::Checksum {
            method: String::from("sha256"),
            hash: version_info.downloads[&loader].file_info.sha256.clone(),
        }),
        dependencies,
    };

    Ok(info)
}
