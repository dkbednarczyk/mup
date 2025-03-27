use std::{
    fs::{self, File},
    io::Read,
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

            let mut current_lockfile = File::open(LOCKFILE_PATH)?;

            let mut contents = String::new();
            current_lockfile.read_to_string(&mut contents)?;

            return Ok(serde_json::from_str(&contents)?);
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

        if self.get(slug).is_err() {
            return Err(anyhow!("project {slug} does not exist in the lockfile"));
        }

        let mut plugins = self.mods.iter();

        let idx = plugins
            .position(|p| p.name == slug)
            .ok_or_else(|| anyhow!("{slug} does not exist in the lockfile"))?;

        let entry = self.mods[idx].clone();

        let mut to_remove = vec![entry.name];

        if let Some(deps) = &entry.dependencies {
            for dep in deps {
                if !remove_orphans {
                    break;
                }

                let cant_be_removed = plugins.any(|p| {
                    let is_different = p.name != slug;
                    let requires_dep = deps.iter().any(|d| d == dep && d.required);

                    is_different && requires_dep
                });

                if !cant_be_removed {
                    to_remove.push(dep.id.clone());
                }
            }
        }

        for slug in to_remove {
            let idx = self
                .mods
                .iter()
                .position(|p| p.name == slug || p.id == slug)
                .ok_or_else(|| anyhow!("{slug} does not exist in the lockfile"))?;

            if !keep_jarfile {
                fs::remove_file(self.mods[idx].get_file_path(&self.loader))?;
            }

            self.mods.remove(idx);
        }

        self.save()?;

        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        let minecraft_version = &self.loader.minecraft_version;

        let version = Versioning::new(minecraft_version).unwrap();

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
