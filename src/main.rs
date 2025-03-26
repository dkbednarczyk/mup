#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

mod loader;
mod plugin;
mod server;

#[derive(Debug, Parser)]
#[command(author = "Damian Bednarczyk <damian@bednarczyk.xyz>")]
#[command(version = "0.1.0")]
#[command(about = "A swiss army knife for Minecraft servers.")]
#[command(arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, action)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Download a modloader jarfile
    #[clap(alias = "l")]
    Loader {
        /// Name of the loader to download
        #[arg(short, long, value_name = "loader", value_parser = loader::Loader::parse_name)]
        name: String,

        /// Minecraft version to target
        #[arg(short, long, default_value = "latest")]
        minecraft_version: String,

        /// Loader version to target
        #[arg(short, long, default_value = "latest")]
        version: String,
    },

    /// Work with plugins and mods
    #[command(subcommand)]
    #[clap(alias = "p")]
    Plugin(plugin::Plugin),

    /// Initialize and configure a server
    #[command(subcommand)]
    #[clap(alias = "s")]
    Server(server::Server),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        unsafe {
            env::set_var("RUST_LOG", String::from("info"));
        }
    }

    pretty_env_logger::init();

    match &cli.command {
        Some(Commands::Loader {
            name,
            minecraft_version,
            version,
        }) => loader::Loader::new(name, minecraft_version, version).fetch()?,
        Some(Commands::Plugin(p)) => plugin::action(p)?,
        Some(Commands::Server(s)) => server::action(s)?,
        _ => (),
    }

    Ok(())
}
