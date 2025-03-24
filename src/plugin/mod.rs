use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Subcommand;
use log::warn;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};

use crate::{loader::Loader, server::lockfile::Lockfile};

mod hangar;
mod modrinth;

#[derive(Debug, Subcommand)]
pub enum Plugin {
    /// Add mods or plugins, including its dependencies
    Add {
        /// The project ID or slug
        #[clap(alias = "slug")]
        id: String,

        /// Which provider to download dependencies from
        #[arg(short, long, default_value = "modrinth", value_parser = ["modrinth", "hangar"])]
        provider: Option<String>,

        /// The version to target.
        /// For Modrinth plugins, this is the version ID.
        #[arg(short, long, default_value = "latest")]
        version: Option<String>,

        /// Also install optional dependencies
        #[arg(short, long, action)]
        optional_deps: bool,

        /// Do not install any dependencies
        #[arg(short, long, action)]
        no_deps: bool,
    },
    /// Remove mods or plugins
    Remove {
        /// The project ID or slug
        id: String,

        /// Keep the downloaded jarfile
        #[arg(long, action)]
        keep_jarfile: bool,

        /// Remove orphans (dependencies which are not required by anything after removal)
        #[arg(long, action)]
        remove_orphans: bool,
    },
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Info {
    pub slug: String,
    pub id: String,
    pub version: String,
    pub dependencies: Vec<Dependency>,
    pub source: String,
    pub url: String,
    pub checksum: Option<String>,
}

impl Info {
    pub fn get_file_path(&self, loader: &Loader) -> PathBuf {
        let filename = self.url.rsplit_once('/').unwrap().1;
        let formatted = format!("{}/{}", loader.mod_location(), filename);

        formatted.into()
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Dependency {
    #[serde(alias = "project_id")]
    pub id: String,
    #[serde(skip)]
    pub required: bool,
}

impl PartialEq for Dependency {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub fn action(plugin: &Plugin) -> Result<()> {
    match plugin {
        Plugin::Add {
            id,
            provider,
            version,
            optional_deps,
            no_deps,
        } => {
            let provider = provider.as_ref().unwrap();
            let version = version.as_ref().unwrap();

            add(provider, id, version, *optional_deps, *no_deps)?;
        }
        Plugin::Remove {
            id,
            keep_jarfile,
            remove_orphans,
        } => remove(id, *keep_jarfile, *remove_orphans)?,
    }

    Ok(())
}

pub fn add(
    provider: &str,
    project_id: &str,
    version: &str,
    optional_deps: bool,
    no_deps: bool,
) -> Result<()> {
    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "you must initialize a server before modifying projects"
        ));
    }

    let info: Result<Info> = match provider {
        "modrinth" => modrinth::fetch(&lockfile, project_id, version),
        "hangar" => hangar::fetch(&lockfile, project_id, version),
        _ => unimplemented!(),
    };

    if let Some(error) = info.as_ref().err() {
        if &error.to_string() == "client side" {
            warn!("project {project_id} does not support server side, skipping");
            return Ok(());
        }

        return Err(info.err().unwrap());
    }

    let info = info.unwrap();

    for dep in &info.dependencies {
        if no_deps {
            break;
        }

        if !dep.required && !optional_deps {
            continue;
        }

        add(provider, &dep.id, "latest", false, false)?;
    }

    lockfile.add(info)
}

pub fn download_plugin(lockfile: &Lockfile, info: &Info) -> Result<()> {
    let file_path = info.get_file_path(&lockfile.loader);

    info.checksum.as_ref().map_or_else(
        || mup::download(&info.url, &file_path),
        |checksum| {
            let (method, hash) = checksum.split_once('#').unwrap();

            match method {
                "sha256" => mup::download_with_checksum::<Sha256>(&info.url, &file_path, hash),
                "sha512" => mup::download_with_checksum::<Sha512>(&info.url, &file_path, hash),
                _ => unimplemented!(),
            }
        },
    )
}

fn remove(id: &str, keep_jarfile: bool, remove_orphans: bool) -> Result<()> {
    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "you must initialize a server before modifying projects"
        ));
    }

    lockfile.remove(id, keep_jarfile, remove_orphans)
}
