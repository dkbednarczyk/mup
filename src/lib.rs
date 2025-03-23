use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use log::info;

pub const FAKE_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.3";

pub fn download(url: &str, path: &Path) -> Result<()> {
    let mut resp = ureq::get(url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?;

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
    info!("downloading jarfile from {url}");

    let mut resp = ureq::get(url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?;

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
    ureq::get(url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_json::<T>()
}

pub fn get_string(url: &str) -> Result<String, ureq::Error> {
    ureq::get(url)
        .header("User-Agent", FAKE_USER_AGENT)
        .call()?
        .body_mut()
        .read_to_string()
}
