//! the `resource` subcommand: convert a png to the device `.raw` screen format,
//! display a `.raw` image in a window, or upload the screen images to a device.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};

use crate::common::{run_command, Globals};

#[derive(Subcommand)]
pub enum ResourceCommand {
    /// convert a png to the device's `.raw` screen format
    Convert { src: PathBuf, dst: PathBuf },
    /// display a `.raw` screen image in a window
    Display { path: PathBuf },
    /// upload screen images to the device
    ///
    /// each file's slot is its numeric filename stem (`23.raw` -> slot 23; `.png`
    /// converted on the fly). with no paths, uploads the bundled base images.
    Upload { images: Vec<PathBuf> },
}

pub fn run(globals: &Globals, command: ResourceCommand) -> Result<()> {
    match command {
        ResourceCommand::Convert { src, dst } => {
            let png = std::fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
            let raw = sextant_core::image::encode_png(&png)
                .with_context(|| format!("converting {}", src.display()))?;
            std::fs::write(&dst, raw).with_context(|| format!("writing {}", dst.display()))?;
            println!("converted {} -> {}", src.display(), dst.display());
            Ok(())
        }
        ResourceCommand::Display { path } => {
            let raw =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            sextant_sim::display::show_image(raw);
            Ok(())
        }
        ResourceCommand::Upload { images } => {
            let parsed = parse_upload_images(&images)?;
            run_command(globals, move |c| {
                c.wait_for_layout();
                if parsed.is_empty() {
                    c.configure_images();
                } else {
                    c.upload_images(&parsed);
                }
                Ok(())
            })
        }
    }
}

/// the device slot for an upload image: its numeric filename stem (`23.raw` -> 23).
fn slot_for(path: &Path) -> Result<i64> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
        .with_context(|| format!("image filename must be a numeric slot: {}", path.display()))
}

/// read each upload image into `(slot, raw)`, converting `.png` sources on the fly.
fn parse_upload_images(paths: &[PathBuf]) -> Result<Vec<(i64, Vec<u8>)>> {
    paths
        .iter()
        .map(|path| {
            let slot = slot_for(path)?;
            let bytes =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            let raw = if path.extension().and_then(|e| e.to_str()) == Some("png") {
                sextant_core::image::encode_png(&bytes)
                    .with_context(|| format!("converting {}", path.display()))?
            } else {
                bytes
            };
            Ok((slot, raw))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_is_the_numeric_filename_stem() {
        assert_eq!(23, slot_for(Path::new("foo/23.raw")).unwrap());
        assert_eq!(0, slot_for(Path::new("0.png")).unwrap());
        assert!(slot_for(Path::new("brand.raw")).is_err());
    }
}
