use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

mod fabric;
mod forge;
mod neoforge;
mod paper;
mod vanilla;

#[derive(Deserialize, Serialize)]
pub struct Loader {
    pub name: String,
    pub minecraft_version: String,
    pub version: String,
    pub snapshot: bool,
}

impl Default for Loader {
    fn default() -> Self {
        Self {
            name: "none".to_string(),
            minecraft_version: "latest".to_string(),
            version: "latest".to_string(),
            snapshot: false,
        }
    }
}

impl Loader {
    const VALID_LOADERS: [&str; 4] = ["paper", "fabric", "forge", "neoforge"];

    pub fn new(loader: &str, minecraft_version: &str, version: &str, snapshot: bool) -> Self {
        Self {
            name: loader.to_string(),
            minecraft_version: minecraft_version.to_string(),
            version: version.to_string(),
            snapshot,
        }
    }

    pub fn fetch(&self) -> Result<()> {
        match self.name.as_str() {
            "paper" => paper::fetch(&self.minecraft_version, &self.version),
            "fabric" => fabric::fetch(&self.minecraft_version, &self.version),
            "forge" => forge::fetch(&self.minecraft_version, &self.version),
            "neoforge" => neoforge::fetch(&self.minecraft_version),
            "vanilla" => vanilla::fetch(&self.minecraft_version, self.snapshot),
            _ => Ok(()),
        }
    }

    pub fn mod_location(&self) -> &str {
        match self.name.as_str() {
            "paper" => "plugins",
            _ => "mods",
        }
    }

    pub fn parse_name(input: &str) -> Result<String> {
        if !Self::VALID_LOADERS.contains(&input) {
            return Err(anyhow!("try one of {:?}", Self::VALID_LOADERS));
        }

        Ok(input.into())
    }
}
