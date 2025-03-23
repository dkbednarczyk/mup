use std::{fs::File, io::{Read, Write}, path::Path};

use anyhow::{anyhow, Result};
use log::info;

pub const FAKE_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.3";

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
