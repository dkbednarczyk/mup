use std::{collections::HashMap, fs::File, io};

use anyhow::anyhow;
use once_cell::sync::Lazy;
use serde::Deserialize;
use versions::Versioning;

const PROMOS_URL: &str =
    "https://files.minecraftforge.net/maven/net/minecraftforge/forge/promotions_slim.json";
const BASE_MAVEN_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

// Forge does not provide installer jarfiles before Minecraft version 1.5.2
static MINECRAFT_CUTOFF: Lazy<Versioning> = Lazy::new(|| Versioning::new("1.5.2").unwrap());

// The cutoff in 1.9 builds after which versions are formatted as 1.X-[installer]-1.X.0
static LOADER_CUTOFF_TRIPLE: Lazy<Versioning> =
    Lazy::new(|| Versioning::new("12.16.1.1938").unwrap());

// The cutoff in 1.9 builds before which versions are formatted as 1.9-[installer]
static LOADER_CUTOFF_DOUBLE: Lazy<Versioning> =
    Lazy::new(|| Versioning::new("12.16.0.1885").unwrap());

#[derive(Debug, Deserialize)]
struct PromosResponse {
    promos: HashMap<String, String>,
}

pub fn fetch(
    minecraft_version: &str,
    installer_version: &str,
    force_latest: bool,
) -> Result<(), anyhow::Error> {
    let promos = get_promos()?;

    let minecraft = if minecraft_version == "latest" {
        let mut versions: Vec<Versioning> = promos
            .keys()
            .filter_map(|p| p.split('-').next())
            .filter_map(Versioning::new)
            .collect();

        versions.sort_by(Versioning::cmp);

        versions.last().unwrap().clone()
    } else {
        Versioning::new(minecraft_version).unwrap()
    };

    let installer_version = if force_latest {
        "latest"
    } else {
        installer_version
    };

    let formatted_version = format!("{minecraft_version}-{installer_version}",);

    let promo = promos.get(&formatted_version);

    let installer = match installer_version {
        "latest" => promo.ok_or_else(|| {
            anyhow!("failed to get the latest installer, is this a valid Minecraft version?")
        })?,
        "recommended" => promo.ok_or_else(|| anyhow!("failed to find a recommended installer"))?,
        _ => installer_version,
    };

    let formatted_url = get_formatted_url(&minecraft, installer)?;

    let resp = ureq::get(&formatted_url)
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?;

    let filename = format!("forge-{minecraft_version}-{installer}.jar");

    let mut file = File::create(filename)?;
    io::copy(&mut resp.into_reader(), &mut file)?;

    Ok(())
}

fn get_promos() -> Result<HashMap<String, String>, anyhow::Error> {
    let resp: PromosResponse = ureq::get(PROMOS_URL)
        .set("User-Agent", pap::FAKE_USER_AGENT)
        .call()?
        .into_json()?;

    Ok(resp.promos)
}

fn get_formatted_url(minecraft: &Versioning, loader: &str) -> Result<String, anyhow::Error> {
    let tag: String = get_version_tag(minecraft, loader)?;

    let formatted_url = format!("{BASE_MAVEN_URL}/{tag}/forge-{tag}-installer.jar");

    Ok(formatted_url)
}

// Did I mention already how much I hate the Forge versioning scheme?
fn get_version_tag(minecraft: &Versioning, loader: &str) -> Result<String, anyhow::Error> {
    if minecraft < &MINECRAFT_CUTOFF {
        return Err(anyhow!(
            "forge does not provide installer jarfiles before Minecraft 1.5.2"
        ));
    }

    match minecraft {
        Versioning::Ideal(s) => {
            if !(7..10).contains(&s.minor) {
                return Ok(format!("{s}-{loader}"));
            }

            if s.minor == 7 && s.patch == 2 {
                return Ok(format!("1.7.2-{loader}-mc172"));
            }

            Ok(format!("{s}-{loader}-{s}"))
        }
        Versioning::General(v) => {
            let minor: u32 = v.chunks.0[1].to_string().parse()?;

            let loader = Versioning::new(loader).unwrap();

            if (9..11).contains(&minor) && &loader >= &LOADER_CUTOFF_TRIPLE {
                return Ok(format!("{v}-{loader}-{v}.0"));
            }

            if minor == 9 && &loader <= &LOADER_CUTOFF_DOUBLE {
                return Ok(format!("{v}-{loader}-{v}"));
            }

            Ok(format!("{v}-{loader}"))
        }
        // This is currently the only release that ends up down here...
        Versioning::Complex(_) => Ok(format!("1.7.10_pre4-{loader}-prerelease")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideal_version() -> Result<(), anyhow::Error> {
        let minecraft = Versioning::new("1.9.4").unwrap();
        let installer = String::from("12.17.0.2317");
        let expected = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.9.4-12.17.0.2317-1.9.4/forge-1.9.4-12.17.0.2317-1.9.4-installer.jar";

        assert_eq!(expected, get_formatted_url(&minecraft, &installer)?);

        Ok(())
    }

    #[test]
    fn test_general_version() -> Result<(), anyhow::Error> {
        let minecraft = Versioning::new("1.9").unwrap();
        let installer = String::from("12.16.1.1938");
        let expected = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.9-12.16.1.1938-1.9.0/forge-1.9-12.16.1.1938-1.9.0-installer.jar";

        assert_eq!(expected, get_formatted_url(&minecraft, &installer)?);

        Ok(())
    }

    #[test]
    fn test_cutoff() -> Result<(), anyhow::Error> {
        let minecraft = Versioning::new("1.2.5").unwrap();
        let installer = String::from("who cares");
        let expected: Result<(), anyhow::Error> = Err(anyhow!(
            "forge does not provide installer jarfiles before Minecraft 1.5.2"
        ));

        assert_eq!(
            expected.err().unwrap().to_string(),
            get_formatted_url(&minecraft, &installer)
                .err()
                .unwrap()
                .to_string()
        );

        Ok(())
    }
}
