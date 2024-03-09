use std::collections::HashMap;

use crate::server::lockfile::Lockfile;

use anyhow::{anyhow, Result};
use log::info;
use mup::FAKE_USER_AGENT;
use serde::Deserialize;
use versions::Versioning;

const BASE_URL: &str = "https://hangar.papermc.io/api/v1";

#[derive(Deserialize)]
struct VersionInfo {
    downloads: HashMap<String, Download>,
    #[serde(rename = "pluginDependencies")]
    dependencies: HashMap<String, Vec<HDependency>>,
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
struct HDependency {
    name: String,
    required: bool,
}

#[derive(Deserialize)]
struct ProjectInfo {
    name: String,
}

pub fn fetch(lockfile: &Lockfile, project_id: &str, version: &str) -> Result<super::Info> {
    info!("fetching info of project {project_id}");

    let formatted_url = format!("{BASE_URL}/projects/{project_id}");

    let project_info: ProjectInfo = ureq::get(&formatted_url)
        .set("User-Agent", FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    let project_id = project_info.name;

    let version = if version == "latest" {
        info!("fetching latest version of project {project_id}");

        let formatted_url = format!("{BASE_URL}/projects/{project_id}/latest");

        ureq::get(&formatted_url)
            .set("User-Agent", FAKE_USER_AGENT)
            .query("channel", "Release")
            .call()?
            .into_string()?
    } else {
        version.into()
    };

    info!("fetching info for {project_id} v{version}");

    let formatted_url = format!("{BASE_URL}/projects/{project_id}/versions/{version}");

    let version_info: VersionInfo = ureq::get(&formatted_url)
        .set("User-Agent", FAKE_USER_AGENT)
        .call()?
        .into_json()?;

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
        .filter_map(|v| Versioning::new(v))
        .any(|v| v == minecraft_version);

    if !is_compatible {
        return Err(anyhow!("version {version} of {project_id} is incompatible with Minecraft version {minecraft_version}"));
    }

    let dependencies = if version_info.dependencies.contains_key(&loader) {
        version_info.dependencies[&loader]
            .iter()
            .map(|d| super::Dependency {
                id: d.name.clone(),
                required: d.required,
            })
            .collect()
    } else {
        vec![]
    };

    let info = super::Info {
        slug: project_id.clone(),
        id: project_id,
        version,
        source: format!("hangar#{}", version_info.downloads[&loader].url),
        checksum: Some(format!(
            "sha256#{}",
            version_info.downloads[&loader].file_info.sha256
        )),
        dependencies,
    };

    Ok(info)
}
