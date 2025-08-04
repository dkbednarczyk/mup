use std::{
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use log::{info, warn};
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

        let mv = Versioning::new(minecraft_version)
            .ok_or_else(|| anyhow!("invalid minecraft version: {minecraft_version}"))?;
        if mv.is_complex() {
            return Err(anyhow!(
                "minecraft version {} is invalid",
                minecraft_version
            ));
        }

        let loader = loader::Loader::new(loader_name, minecraft_version, "latest", false);

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
            .find(|p| p.name == project_id || p.id == project_id)
            .ok_or_else(|| anyhow!("key {project_id} not found"))
    }

    pub fn add(&mut self, info: plugin::Info) -> Result<()> {
        if let Some(idx) = self
            .mods
            .iter()
            .position(|p| p.id == info.id || p.name == info.name)
        {
            self.mods[idx] = info;
        } else {
            self.mods.push(info);
        }

        self.save()?;

        Ok(())
    }

    pub fn remove(&mut self, slug: &str, keep_jarfile: bool) -> Result<()> {
        info!("removing {slug} from lockfile");

        let entry = self.get(slug)?;

        if !keep_jarfile {
            let path = entry.get_file_path(&self.loader);
            info!("removing {}", path.to_string_lossy());

            if let Err(e) = fs::remove_file(path) {
                warn!("failed to remove jarfile for {slug}: {e}");
            }
        }

        let entry_idx = self
            .mods
            .iter()
            .position(|p| p.id == slug || p.name == slug)
            .ok_or_else(|| anyhow!("key {slug} not found"))?;

        self.mods.remove(entry_idx);

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
