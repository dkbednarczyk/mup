use anyhow::{anyhow, Result};
use clap::Subcommand;

mod eula;
pub mod lockfile;

use lockfile::Lockfile;

use crate::{loader, plugin};

#[derive(Debug, Subcommand)]
pub enum Server {
    /// Initialize a server in the current directory
    Init {
        /// Minecraft version of the server
        #[arg(short, long, required = true)]
        minecraft_version: String,

        /// Which loader to use
        #[arg(short, long, required = true, value_parser = loader::Loader::parse_name)]
        loader: String,
    },

    /// Sign the eula.txt
    Sign,

    /// Install all mods from the current lockfile
    Install,
}

pub fn action(server: &Server) -> Result<()> {
    match server {
        Server::Init {
            minecraft_version,
            loader,
        } => init(minecraft_version, loader),
        Server::Sign => eula::sign(),
        Server::Install => install(),
    }
}

fn init(minecraft_version: &str, loader: &str) -> Result<()> {
    let lf = Lockfile::with_params(minecraft_version, loader)?;

    if !lf.is_initialized() {
        return Err(anyhow!(
            "lockfile was initialized with invalid configuration"
        ));
    }

    lf.loader.fetch()?;

    eula::sign()?;

    Ok(())
}

fn install() -> Result<()> {
    let lf = Lockfile::init()?;
    if !lf.is_initialized() {
        return Err(anyhow!("failed to read lockfile"));
    }

    lf.loader.fetch()?;

    for entry in &lf.plugins {
        plugin::download_plugin(&lf, entry)?;
    }

    eula::sign()?;

    Ok(())
}
