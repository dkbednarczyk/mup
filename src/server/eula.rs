use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use log::info;

pub fn sign() -> Result<()> {
    info!("signing eula");

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    #[test]
    fn test_sign_eula() -> Result<()> {
        let dir = tempdir()?;
        let original_dir = env::current_dir()?;
        env::set_current_dir(&dir)?;

        sign()?;

        let content = fs::read_to_string("eula.txt")?;
        assert_eq!(content, "# Signed by mup\neula=true");

        env::set_current_dir(original_dir)?;
        dir.close()?;

        Ok(())
    }
}
