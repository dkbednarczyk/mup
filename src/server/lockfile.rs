use std::{
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use log::info;
use serde::{Deserialize, Serialize};
use versions::Versioning;

use crate::{loader, plugin};

const LOCKFILE_PATH: &str = "mup.lock.json";

#[derive(Deserialize, Default, Serialize)]
pub struct Lockfile {
    pub loader: loader::Loader,
    pub mods: Vec<plugin::Info>,
}

impl Lockfile {
    pub fn init() -> Result<Self> {
        info!("initializing lockfile");

        if PathBuf::from(LOCKFILE_PATH).exists() {
            info!("using existing lockfile");

            let current_lockfile = File::open(LOCKFILE_PATH)?;

            return Ok(serde_json::from_reader(&current_lockfile)?);
        }

        info!("creating new lockfile");

        File::create(LOCKFILE_PATH)?;

        Ok(Self {
            loader: loader::Loader::default(),
            mods: vec![],
        })
    }

    pub fn with_params(minecraft_version: &str, loader_name: &str) -> Result<Self> {
        info!("initializing lockfile with Minecraft version {minecraft_version} and loader {loader_name}");

        let mv = Versioning::new(minecraft_version).unwrap();
        if mv.is_complex() {
            return Err(anyhow!(
                "minecraft version {} is invalid",
                minecraft_version
            ));
        }

        let loader = loader::Loader::new(loader_name, minecraft_version, "latest");

        File::create(LOCKFILE_PATH)?;

        let lf = Self {
            loader,
            mods: vec![],
        };

        lf.save()?;

        Ok(lf)
    }

    pub fn get(&self, project_id: &str) -> Result<&plugin::Info> {
        self.mods
            .iter()
            .find(|p| p.name == project_id)
            .ok_or_else(|| anyhow!("key {project_id} not found"))
    }

    pub fn add(&mut self, info: plugin::Info) -> Result<()> {
        self.mods.push(info);

        self.save()?;

        Ok(())
    }

    pub fn remove(&mut self, slug: &str, keep_jarfile: bool, remove_orphans: bool) -> Result<()> {
        info!("removing {slug} from lockfile");

        let entry = self.get(slug)?;
        let mut to_remove = vec![slug.to_string()];

        if remove_orphans && entry.dependencies.is_some() {
            let deps = entry.dependencies.as_ref().unwrap();

            for dep in deps {
                let is_required_by_something_else = self.mods.iter().any(|p| {
                    p.name != slug
                        && p.id != slug
                        && p.dependencies
                            .as_ref()
                            .is_some_and(|p_deps| p_deps.contains(dep))
                });

                if !is_required_by_something_else {
                    to_remove.push(dep.id.clone());
                }
            }
        }

        let mods_to_remove = to_remove
            .iter()
            .map(|slug| {
                self.mods
                    .iter()
                    .find(|p| p.name == *slug || p.id == *slug)
                    .ok_or_else(|| anyhow!("{slug} does not exist in the lockfile"))
            })
            .collect::<Result<Vec<_>>>()?;

        if !keep_jarfile {
            for mod_item in &mods_to_remove {
                let path = mod_item.get_file_path(&self.loader);
                info!("removing {}", path.to_string_lossy());
                fs::remove_file(path)?;
            }
        }

        self.mods.retain(|p| {
            !to_remove
                .iter()
                .any(|slug| p.name == *slug || p.id == *slug)
        });

        self.save()?;

        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        let version = Versioning::new(&self.loader.minecraft_version).unwrap();

        !version.is_complex() && self.loader.name != "none"
    }

    pub fn save(&self) -> Result<()> {
        info!("saving transaction to lockfile");

        let mut output = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(LOCKFILE_PATH)?;

        serde_json::to_writer_pretty(&mut output, &self)?;

        Ok(())
    }
}
