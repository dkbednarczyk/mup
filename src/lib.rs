use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use anyhow::{anyhow, Result};
use log::info;
use sha2::Digest;
use ureq::{typestate::WithoutBody, RequestBuilder};

pub const USER_AGENT: &str = concat!(
    "dkbednarczyk/mup/",
    env!("CARGO_PKG_VERSION"),
    " (damian@bednarczyk.xyz)"
);

pub fn download(url: &str, path: &Path) -> Result<()> {
    info!(
        "downloading {} from {url}",
        path.to_str().ok_or_else(|| anyhow!("invalid path"))?
    );

    if let Some(prefix) = path.parent() {
        std::fs::create_dir_all(prefix)?;
    }

    let mut resp = get(url).call()?;

    let mut file = File::create(path)?;
    io::copy(&mut resp.body_mut().as_reader(), &mut file)?;

    Ok(())
}

fn hash_and_write<R: Read, W: Write, D: Digest + Write>(
    mut reader: R,
    mut writer: W,
) -> Result<String> {
    let mut hasher = D::new();
    let mut buf = [0; 1024];

    loop {
        let count = reader.read(&mut buf)?;
        if count == 0 {
            break;
        }

        writer.write_all(&buf[..count])?;
        hasher.write_all(&buf[..count])?;
    }

    let digest = hasher.finalize();
    let hash = digest
        .as_slice()
        .iter()
        .fold(String::new(), |acc, b| acc + &format!("{b:02x}"));

    Ok(hash)
}

pub fn download_with_checksum<T: Digest + Write>(
    url: &str,
    path: &Path,
    wanted_hash: &str,
) -> Result<()> {
    info!(
        "downloading {} from {url} with expected hash {wanted_hash}",
        path.to_str().ok_or_else(|| anyhow!("invalid path"))?
    );

    let mut resp = get(url).call()?;
    let body = resp.body_mut().as_reader();

    if let Some(prefix) = path.parent() {
        std::fs::create_dir_all(prefix)?;
    }

    let output = File::create(path)?;
    let hash = hash_and_write::<_, _, T>(body, output)?;

    if hash != wanted_hash {
        return Err(anyhow!(
            "hashes do not match, expected {wanted_hash} but got {hash}"
        ));
    }

    Ok(())
}

pub fn get(url: &str) -> RequestBuilder<WithoutBody> {
    ureq::get(url).header("User-Agent", USER_AGENT)
}

pub fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, ureq::Error> {
    info!("fetching json from {url}");

    get(url).call()?.body_mut().read_json::<T>()
}

pub fn get_string(url: &str) -> Result<String, ureq::Error> {
    info!("fetching string from {url}");

    get(url).call()?.body_mut().read_to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::Sha1;

    #[test]
    fn test_hash_and_write() -> Result<()> {
        let input_data = b"hello world";
        let expected_hash = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";

        let reader = std::io::Cursor::new(input_data);
        let writer = Vec::new();

        let hash = hash_and_write::<_, _, Sha1>(reader, writer)?;
        assert_eq!(hash, expected_hash);

        Ok(())
    }
}
