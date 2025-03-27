use std::{fmt, path::PathBuf};

use anyhow::{anyhow, Result};
use clap::Subcommand;
use log::info;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize,
};
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
        provider: String,

        /// The version to target.
        /// For Modrinth plugins, this is the version ID.
        #[arg(short, long, default_value = "latest")]
        version: String,

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
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
    pub download_url: String,

    pub dependencies: Option<Vec<Dependency>>,
    pub checksum: Option<Checksum>,
}

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Dependency {
    #[serde(alias = "project_id")]
    pub id: String,
    pub version: Option<String>,
    #[serde(deserialize_with = "bool_from_string", alias = "dependency_type")]
    pub required: bool,
}

// Modrinth returns dependency requirements as strings
// We assume "required" is true and anything else is false
// This is also used for deserializing the lockfile itself
fn bool_from_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    struct BoolOrString;

    impl Visitor<'_> for BoolOrString {
        type Value = bool;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a boolean or a string")
        }

        fn visit_bool<E: de::Error>(self, value: bool) -> Result<bool, E> {
            Ok(value)
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<bool, E> {
            match value {
                "required" => Ok(true),
                _ => Ok(false),
            }
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<bool, E> {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(BoolOrString)
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
    info!("adding {project_id} version {version} from {provider}");

    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "you must initialize a server before modifying projects"
        ));
    }

    if lockfile.get(project_id).is_ok() {
        return Err(anyhow!("project {project_id} is already installed"));
    }

    let info = match provider {
        "modrinth" => modrinth::fetch(&lockfile, project_id, version)?,
        "hangar" => hangar::fetch(&lockfile, project_id, version)?,
        _ => unimplemented!(),
    };

    if let Some(deps) = &info.dependencies {
        for dep in deps {
            if no_deps {
                break;
            }

            if !dep.required && !optional_deps {
                continue;
            }

            add(provider, &dep.id, "latest", false, false)?;
        }
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

fn remove(id: &str, keep_jarfile: bool, remove_orphans: bool) -> Result<()> {
    let mut lockfile = Lockfile::init()?;

    if !lockfile.is_initialized() {
        return Err(anyhow!(
            "you must initialize a server before modifying projects"
        ));
    }

    lockfile.remove(id, keep_jarfile, remove_orphans)
}
