use std::{fs::File, io, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::Subcommand;
use log::warn;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};

use crate::{loader, server::lockfile::Lockfile};

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
    pub checksum: Option<String>,
}

impl Info {
    pub fn get_file_path(&self, l: &str) -> String {
        let filename = self.source.rsplit_once('/').unwrap().1;
        format!("{}/{}", loader::location(l), filename)
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

    download(&info.source, &lockfile.loader.name, info.checksum.as_ref())?;

    lockfile.add(info)
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

pub fn download(source: &str, loader_name: &str, checksum: Option<&String>) -> Result<()> {
    let filename = source.rsplit_once('/').unwrap().1;
    let file_path = format!("{}/{}", loader::location(loader_name), filename);

    let source = source.split_once('#').unwrap().1;

    if checksum.is_none() {
        let mut resp = ureq::get(source)
            .header("User-Agent", mup::FAKE_USER_AGENT)
            .call()?;

        let mut body = resp.body_mut().as_reader();

        let mut file = File::create(&file_path)?;
        io::copy(&mut body, &mut file)?;
    }

    let (method, hash) = checksum.unwrap().split_once('#').unwrap();

    match method {
        "sha512" => mup::download_with_checksum::<Sha512>(source, &PathBuf::from(file_path), hash),
        "sha256" => mup::download_with_checksum::<Sha256>(source, &PathBuf::from(file_path), hash),
        _ => unimplemented!(),
    }
}
