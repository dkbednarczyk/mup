use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use log::info;

pub fn sign() -> Result<()> {
    let mut file = if fs::metadata("eula.txt").is_err() {
        info!("creating eula.txt");
        File::create("eula.txt")?
    } else {
        info!("overwriting eula.txt");
        fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open("eula.txt")?
    };

    file.write_all(b"# Signed by mup\neula=true")?;

    Ok(())
}
