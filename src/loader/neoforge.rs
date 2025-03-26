use std::{path::Path, sync::LazyLock};

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::Deserialize;
use versions::SemVer;

const API_URL: &str =
    "https://maven.neoforged.net/api/maven/latest/version/releases/net/neoforged/neoforge";
const DOWNLOAD_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

static CUTOFF: LazyLock<SemVer> = LazyLock::new(|| SemVer::new("1.20.2").unwrap());

#[derive(Deserialize)]
struct Installer {
    version: String,
}

// see https://github.com/neoforged/websites/blob/main/assets/js/neoforge.js
pub fn fetch(minecraft_version: &str) -> Result<()> {
    let mut endpoint = API_URL.to_string();

    if minecraft_version != "latest" {
        let version = SemVer::new(minecraft_version)
            .ok_or_else(|| anyhow!("invalid minecaft version {minecraft_version}"))?;

        if version < *CUTOFF {
            return Err(anyhow!("use forge for minecraft versions before 1.20.2"));
        }

        let double = format!("{}.{}", version.minor, version.patch);

        endpoint += &format!("?filter={double}");
    }

    info!("fetching latest installer version for minecraft {minecraft_version}");

    let installer: Installer = mup::get_json(&endpoint)?;

    let installer_url = format!(
        "{DOWNLOAD_URL}/{}/neoforge-{}-installer.jar",
        installer.version, installer.version
    );
    let filename = format!("neoforge-{minecraft_version}-{}.jar", installer.version);

    info!("downloading installer jarfile");

    mup::download(&installer_url, Path::new(&filename))?;

    warn!("neoforge servers must be installed manually using the downloaded jarfile");

    Ok(())
}
