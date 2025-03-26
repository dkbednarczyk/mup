use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use log::info;

pub fn sign() -> Result<()> {
    info!("overwriting eula.txt");

    let mut file = if fs::metadata("eula.txt").is_err() {
        File::create("eula.txt")?
    } else {
        fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("eula.txt")?
    };

    file.write_all(b"# Signed by mup\neula=true")?;

    Ok(())
}
