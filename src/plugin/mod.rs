use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Subcommand;
use log::info;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};

use crate::{loader::Loader, server::lockfile::Lockfile};

mod hangar;
mod modrinth;

#[derive(Debug, Subcommand)]
pub enum Plugin {
    /// Add mods or plugins and their dependencies
    Add {
        /// The project ID or slug
        #[clap(alias = "slug")]
        id: String,

        /// Which provider to download dependencies from
        #[arg(short, long, default_value = "modrinth", value_parser = ["modrinth", "hangar"])]
        provider: String,

        /// The version to add.
        /// For Modrinth plugins, this is the version ID.
        #[arg(short, long, default_value = "latest")]
        version: String,

        /// Do not install any dependencies
        #[arg(short, long, action)]
        no_deps: bool,
    },
    /// Remove an installed mod or plugin
    Remove {
        /// The project ID or slug
        id: String,

        /// Keep the downloaded jarfile
        #[arg(long, action)]
        keep_jarfile: bool,
    },
    /// Update mods or plugins
    Update {
        /// The project ID or slug
        #[clap(alias = "slug", default_value = "all")]
        id: String,

        /// The version to update to.
        /// For Modrinth plugins, this is the version ID.
        #[arg(short, long, default_value = "latest")]
        version: String,
    },
}

#[derive(Deserialize, Serialize)]
pub struct Info {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
    pub download_url: String,

    pub dependencies: Option<Vec<Dependency>>,
    pub checksum: Option<Checksum>,
}

#[derive(Deserialize, Serialize)]
pub struct Checksum {
    pub method: String,
    pub hash: String,
}

impl Info {
    pub fn get_file_path(&self, loader: &Loader) -> PathBuf {
        let filename = self.download_url.rsplit_once('/').unwrap().1;
        let formatted = format!("{}/{}", loader.mod_location(), filename);

        formatted.into()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dependency {
    #[serde(skip)]
    pub id: String,
    #[serde(skip)]
    pub source: String,
    pub name: String,
    pub required: bool,
}

impl From<modrinth::ModrinthDependency> for Dependency {
    fn from(val: modrinth::ModrinthDependency) -> Self {
        Self {
            id: val.project_id,
            source: "modrinth".to_string(),
            name: val.slug.to_lowercase(),
            required: val.dependency_type == "required",
        }
    }
}

impl From<&hangar::HangarDependency> for Dependency {
    fn from(val: &hangar::HangarDependency) -> Self {
        Self {
            id: val.project_id.to_string(),
            source: "hangar".to_string(),
            name: val.name.to_lowercase(),
            required: val.required,
        }
    }
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        if self.source == other.source {
            self.id == other.id
        } else {
            self.name == other.name
        }
    }
}

pub fn action(plugin: &Plugin) -> Result<()> {
    match plugin {
        Plugin::Add {
            id,
            provider,
            version,
            no_deps,
        } => {
            add(provider, id, version, *no_deps)?;
        }
        Plugin::Remove { id, keep_jarfile } => remove(id, *keep_jarfile)?,
        Plugin::Update { id, version } => update(id, version)?,
    }

    Ok(())
}

pub fn add(provider: &str, project_id: &str, version: &str, no_deps: bool) -> Result<()> {
    info!("adding {project_id} version {version} from {provider}");

    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "Server must be initialized before removing projects"
        ));
    }

    let project = lockfile.get(project_id)?;

    if project.name == project_id && project.version == version {
        return Err(anyhow!(
            "Project '{project_id}' version {version} is already installed"
        ));
    }

    let info = match provider {
        "modrinth" => modrinth::fetch(&lockfile, project_id, version)?,
        "hangar" => hangar::fetch(&lockfile, project_id, version)?,
        _ => unimplemented!(),
    };

    let remove_old_version = project.version != info.version;

    if let Some(deps) = &info.dependencies {
        for dep in deps {
            if no_deps {
                break;
            }

            if !dep.required {
                continue;
            }

            add(provider, &dep.id, "latest", false)?;
        }
    }

    if remove_old_version {
        info!(
            "removing old version of {}: {}",
            project.name, project.version
        );

        remove(&project.name, false)?;
    }

    download_plugin(&lockfile, &info)?;

    lockfile.add(info)
}

pub fn download_plugin(lockfile: &Lockfile, info: &Info) -> Result<()> {
    info!(
        "downloading {} for {} version {}",
        info.name, lockfile.loader.name, info.version
    );

    let file_path = info.get_file_path(&lockfile.loader);

    info.checksum.as_ref().map_or_else(
        || mup::download(&info.download_url, &file_path),
        |checksum| {
            info!(
                "downloading jarfile to {} from {}",
                file_path.to_str().unwrap(),
                info.download_url
            );

            match checksum.method.as_str() {
                "sha256" => mup::download_with_checksum::<Sha256>(
                    &info.download_url,
                    &file_path,
                    &checksum.hash,
                ),
                "sha512" => mup::download_with_checksum::<Sha512>(
                    &info.download_url,
                    &file_path,
                    &checksum.hash,
                ),
                _ => unimplemented!(),
            }
        },
    )
}

fn remove(id: &str, keep_jarfile: bool) -> Result<()> {
    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "Server must be initialized before updating projects"
        ));
    }

    lockfile.remove(id, keep_jarfile)
}

pub fn update(id: &str, version: &str) -> Result<()> {
    let lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "you must initialize a server before modifying projects"
        ));
    }

    if id == "all" {
        for plugin in lockfile.mods {
            update(&plugin.name, version)?;
        }
    } else {
        add("modrinth", id, version, true)?;
    }

    Ok(())
}
