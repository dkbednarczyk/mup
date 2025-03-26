use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use log::info;

pub const USER_AGENT: &str = "dkbednarczyk/mup/0.1.0 (damian@bednarczyk.xyz)";

pub fn download(url: &str, path: &Path) -> Result<()> {
    info!("downloading {} from {url}", path.to_str().unwrap());

    let mut resp = ureq::get(url).header("User-Agent", USER_AGENT).call()?;

    let mut body = resp.body_mut().as_reader();

    let mut file = File::create(path)?;
    io::copy(&mut body, &mut file)?;

    Ok(())
}

pub fn download_with_checksum<T: sha2::Digest + Write>(
    url: &str,
    path: &Path,
    wanted_hash: &str,
) -> Result<()> {
    info!(
        "downloading {} from {url} with expected hash {wanted_hash}",
        path.to_str().unwrap()
    );

    let mut resp = ureq::get(url).header("User-Agent", USER_AGENT).call()?;

    let mut body = resp.body_mut().as_reader();

    if let Some(prefix) = path.parent() {
        std::fs::create_dir_all(prefix).unwrap();
    }

    let mut output = File::create(path)?;

    // read body 1024 bytes at a time, write to file and hash writers at same time
    let digest = {
        let mut hasher = T::new();
        let mut buf = [0; 1024];

        loop {
            let count = body.read(&mut buf)?;
            if count == 0 {
                break;
            }

            _ = output.write(&buf[..count])?;
            hasher.update(&buf[..count]);
        }

        hasher.finalize()
    };

    let hash = digest
        .as_slice()
        .iter()
        .fold(String::new(), |acc, b| acc + &format!("{b:02x}"));

    if hash != wanted_hash {
        return Err(anyhow!("hashes do not match"));
    }

    Ok(())
}

pub fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, ureq::Error> {
    info!("fetching json from {url}");

    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .read_json::<T>()
}

pub fn get_string(url: &str) -> Result<String, ureq::Error> {
    info!("fetching string from {url}");

    ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .body_mut()
        .read_to_string()
}
