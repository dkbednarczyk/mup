use std::collections::HashMap;

use crate::server::lockfile::Lockfile;

use anyhow::{anyhow, Result};
use log::info;
use serde::Deserialize;
use versions::Versioning;

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
    let project_info: ProjectInfo = mup::get_json(&formatted_url)?;

    let project_id = project_info.name;

    let version = if version == "latest" {
        info!("fetching latest version of project {project_id}");

        let formatted_url = format!("{BASE_URL}/projects/{project_id}/latest");

        mup::get_string(&formatted_url)?
    } else {
        version.into()
    };

    info!("fetching info for {project_id} v{version}");

    let formatted_url = format!("{BASE_URL}/projects/{project_id}/versions/{version}");

    let version_info: VersionInfo = mup::get_json(&formatted_url)?;

    let loader = lockfile.loader.name.to_uppercase();

    if !version_info.platform_dependencies.contains_key(&loader) {
        return Err(anyhow!(
            "plugin version {version} does not support {loader}"
        ));
    }

    let minecraft_version = Versioning::new(&lockfile.loader.minecraft_version).unwrap();
    let is_compatible = version_info.platform_dependencies[&loader]
        .iter()
        // Why this doesn't work without the closure I will never know.
        .filter_map(Versioning::new)
        .any(|v| v == minecraft_version);

    if !is_compatible {
        return Err(anyhow!("version {version} of {project_id} is incompatible with Minecraft version {minecraft_version}"));
    }

    let dependencies = if version_info.dependencies.contains_key(&loader) {
        Some(version_info.dependencies[&loader].clone())
    } else {
        None
    };

    let info = super::Info {
        name: project_id.clone(),
        id: project_id,
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
