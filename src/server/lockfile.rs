#![allow(clippy::cast_possible_truncation)]

use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use versions::Versioning;

use crate::project::actions;

const LOCKFILE_PATH: &str = "pap.lock";
const LOCKFILE_MESSAGE: &str = "# This file is automatically @generated by pap.
# Do not edit this file manually, unless you _really_ messed something up.
";

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct Lockfile {
    pub loader: Loader,
    pub project: Vec<Entry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Loader {
    pub name: String,
    pub minecraft_version: String,
    pub version: String,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            name: String::default(),
            minecraft_version: String::from("latest"),
            version: String::from("latest"),
        }
    }
}

impl Loader {
    pub const VALID_LOADERS: [&'static str; 4] = ["fabric", "forge", "paper", "neoforge"];

    pub fn project_path(&self) -> String {
        match self.name.as_str() {
            "fabric" | "forge" => String::from("./mods/"),
            "paper" => String::from("./plugins/"),
            _ => unimplemented!(),
        }
    }

    pub fn list() {
        println!("{}", Self::VALID_LOADERS.join(", "));
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entry {
    pub slug: String,
    pub installed_version: String,
    path: PathBuf,
    remote_url: String,
    sha512: String,
}

impl Lockfile {
    pub fn init() -> Result<Self, anyhow::Error> {
        if PathBuf::from(LOCKFILE_PATH).exists() {
            let mut current_lockfile = File::open(LOCKFILE_PATH)?;

            let lockfile_size = current_lockfile.metadata()?.size();
            let mut contents = String::with_capacity(lockfile_size as usize);

            current_lockfile.read_to_string(&mut contents)?;

            return Ok(toml::from_str(&contents)?);
        }

        File::create(LOCKFILE_PATH)?;

        Ok(Self {
            loader: Loader::default(),
            project: vec![],
        })
    }

    pub fn with_params(minecraft_version: &str, loader: &str) -> Result<Self, anyhow::Error> {
        let mv = Versioning::new(minecraft_version).unwrap();
        if mv.is_complex() {
            return Err(anyhow!(
                "minecraft version {} is invalid",
                minecraft_version
            ));
        }

        let l = Loader {
            name: loader.to_string(),
            minecraft_version: minecraft_version.to_string(),
            version: String::from("latest"),
        };

        File::create(LOCKFILE_PATH)?;

        let mut lf = Self {
            loader: l,
            project: vec![],
        };

        lf.write_out()?;

        Ok(lf)
    }

    pub fn get(&self, project_id: &str) -> Result<&Entry, anyhow::Error> {
        self.project
            .iter()
            .find(|p| p.slug == project_id)
            .ok_or_else(|| anyhow!("key {project_id} not found"))
    }

    pub fn add(
        &mut self,
        version: &actions::Version,
        project: &actions::ProjectInfo,
        project_file: &actions::ProjectFile,
        path: PathBuf,
    ) -> Result<(), anyhow::Error> {
        let entry = Entry {
            slug: project.slug.clone(),
            installed_version: version.id.clone(),
            path,
            remote_url: project_file.url.clone(),
            sha512: project_file.hashes.sha512.clone(),
        };

        self.project.push(entry);

        self.write_out()?;

        Ok(())
    }

    pub fn remove(&mut self, slug: &str, keep_jarfile: bool) -> Result<(), anyhow::Error> {
        let entry = self
            .project
            .iter()
            .position(|p| p.slug == slug)
            .ok_or_else(|| anyhow!("{slug} does not exist in the lockfile"))?;

        if !keep_jarfile {
            fs::remove_file(&self.project[entry].path)?;
        }

        self.project.remove(entry);

        self.write_out()?;

        Ok(())
    }

    pub fn is_initialized(&mut self) -> bool {
        let minecraft_version = &self.loader.minecraft_version;

        let version = Versioning::new(minecraft_version).unwrap();

        !version.is_complex()
    }

    fn write_out(&mut self) -> Result<(), anyhow::Error> {
        let mut output = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(LOCKFILE_PATH)?;

        output.write_all(LOCKFILE_MESSAGE.as_bytes())?;
        output.write_all(toml::to_string(&self)?.as_bytes())?;

        Ok(())
    }
}
