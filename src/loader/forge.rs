use std::{collections::HashMap, fs::File, io, sync::LazyLock};

use anyhow::{anyhow, Result};
use log::{info, warn};
use serde::Deserialize;
use versions::Versioning;

const PROMOS_URL: &str =
    "https://files.minecraftforge.net/maven/net/minecraftforge/forge/promotions_slim.json";
const BASE_MAVEN_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

// Forge does not provide installer jarfiles before Minecraft version 1.5.2
static MINECRAFT_CUTOFF: LazyLock<Versioning> = LazyLock::new(|| Versioning::new("1.5.2").unwrap());

// The cutoff in 1.9 builds after which versions are formatted as 1.X-{installer}-1.X.0
static INSTALLER_CUTOFF_TRIPLE: LazyLock<Versioning> =
    LazyLock::new(|| Versioning::new("12.16.1.1938").unwrap());

// The cutoff in 1.9 builds before which versions are formatted as 1.9-{installer}
static INSTALLER_CUTOFF_DOUBLE: LazyLock<Versioning> =
    LazyLock::new(|| Versioning::new("12.16.0.1885").unwrap());

#[derive(Deserialize)]
struct PromosResponse {
    promos: HashMap<String, String>,
}

pub fn fetch(minecraft_version: &str, installer_version: &str) -> Result<()> {
    info!("fetching promos");

    let promos = ureq::get(PROMOS_URL)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json::<PromosResponse>()?
        .promos;

    let minecraft = if minecraft_version == "latest" {
        promos
            .keys()
            .filter_map(|p| p.split('-').next())
            .filter_map(Versioning::new)
            .max()
            .unwrap()
    } else {
        Versioning::new(minecraft_version).unwrap()
    };

    let installer = if installer_version == "latest" {
        promos
            .get(&format!("{minecraft}-{installer_version}"))
            .ok_or_else(|| anyhow!("invalid or unsupported minecraft version"))?
    } else {
        installer_version
    };

    let version_tag = get_version_tag(&minecraft, installer)?;

    let formatted_url = format!("{BASE_MAVEN_URL}/{version_tag}/forge-{version_tag}-installer.jar");

    info!("downloading installer jarfile");

    let mut resp = ureq::get(&formatted_url)
        .header("User-Agent", mup::FAKE_USER_AGENT)
        .call()?;

    let mut body = resp.body_mut().as_reader();

    let filename = format!("forge-{minecraft}-{installer}.jar");


    let mut file = File::create(filename)?;
    io::copy(&mut body, &mut file)?;

    warn!("this is an installer, not a server loader! please run it and install the server before proceeding.");

    Ok(())
}

fn get_version_tag(minecraft: &Versioning, installer: &str) -> Result<String> {
    if minecraft < &MINECRAFT_CUTOFF {
        return Err(anyhow!(
            "forge does not provide installer jarfiles before Minecraft 1.5.2"
        ));
    }

    // Lots of edge cases here
    match minecraft {
        Versioning::Ideal(s) => {
            if !(7..10).contains(&s.minor) {
                return Ok(format!("{s}-{installer}"));
            }

            if s.minor == 7 && s.patch == 2 {
                return Ok(format!("1.7.2-{installer}-mc172"));
            }

            Ok(format!("{s}-{installer}-{s}"))
        }
        Versioning::General(v) => {
            let minor: u32 = v.chunks.0[1].to_string().parse()?;

            let installer = Versioning::new(installer).unwrap();

            if (9..11).contains(&minor) && &installer >= &INSTALLER_CUTOFF_TRIPLE {
                return Ok(format!("{v}-{installer}-{v}.0"));
            }

            if minor == 9 && &installer <= &INSTALLER_CUTOFF_DOUBLE {
                return Ok(format!("{v}-{installer}-{v}"));
            }

            Ok(format!("{v}-{installer}"))
        }
        // This is currently the only release that ends up down here...
        Versioning::Complex(_) => Ok(format!("1.7.10_pre4-{installer}-prerelease")),
    }
}
