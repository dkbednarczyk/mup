use std::{fs::File, io, sync::LazyLock};

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::Deserialize;
use versions::Versioning;

static CUTOFF: LazyLock<Versioning> =
    LazyLock::new(|| Versioning::new("1.20.1").unwrap());

const API_URL: &str = "https://maven.neoforged.net/api/maven/latest/version/releases/net/neoforged/neoforge";
const DOWNLOAD_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

#[derive(Deserialize)]
struct Installer {
    version: String,
}

// see https://github.com/neoforged/websites/blob/main/assets/js/neoforge.js
pub fn fetch(minecraft_version: &str) -> Result<()> {
    if minecraft_version == "latest" {
        return Err(anyhow!(
            "for neoforge, you must specify a minecraft version to target"
        ));
    }

    let parsed_version = Versioning::new(minecraft_version).unwrap();

    if parsed_version <= *CUTOFF {
        return Err(anyhow!(
            "neoforge is not recommended for Minecraft versions before 1.20.2"
        ));
    }

    info!("fetching latest installer version for minecraft {minecraft_version}");

    let installer: Installer = ureq::get(API_URL)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json()?;

    let installer_url = format!(
        "{DOWNLOAD_URL}/{}/neoforge-{}-installer.jar",
        installer.version, installer.version
    );

    info!("downloading installer jarfile");

    let mut resp = ureq::get(&installer_url)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?;

    let mut body = resp.body_mut().as_reader();

    let filename = format!("neoforge-{minecraft_version}-{}.jar", installer.version);

    let mut file = File::create(filename)?;
    io::copy(&mut body, &mut file)?;

    warn!("this is an installer, not a server loader! please run it and install the server before proceeding.");

    Ok(())
}
