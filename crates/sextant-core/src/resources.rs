//! resource access, split into two roles.
//!
//! the embedded `resources/` tree (profiles, scripts, assets) is compiled into
//! the binary purely as a *seed*: `sextant init` mirrors it into the
//! configuration home, and nothing else reads it. at runtime the profiles,
//! screen images, logo, and font are read from that extracted home
//! (`config_home()`), so a single editable tree is the source of truth. the app
//! requires an explicit `init` -- runtime lookups return `None` (or an error)
//! until the home exists.

use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "resources/"]
struct Resources;

/// the sextant configuration home: `$XDG_CONFIG_HOME/sextant`, falling back to
/// `$HOME/.config/sextant`. `sextant init` mirrors the embedded `resources/`
/// tree here, `--profiles NAME` prefers `<home>/profiles/NAME/`, and the
/// `${SEXTANT_CONFIG}` token in `executeCommand` argv expands to it (see
/// DESIGN.md). a non-absolute `XDG_CONFIG_HOME` is ignored, per the xdg spec.
pub fn config_home() -> PathBuf {
    config_home_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// pure resolver behind [`config_home`], separated so the env precedence is
/// testable without mutating the process environment.
fn config_home_from(xdg: Option<OsString>, home: Option<OsString>) -> PathBuf {
    xdg.map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home.map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("sextant")
}

/// every embedded resource path (relative to `resources/`), sorted. this is the
/// seed `sextant init` walks to mirror the tree into the configuration home; it
/// is the only runtime use of the embedded data.
pub fn resource_paths() -> Vec<String> {
    let mut paths: Vec<String> = Resources::iter().map(|p| p.into_owned()).collect();
    paths.sort();
    paths
}

/// bytes of an embedded (seed) resource by its path within `resources/`. used by
/// `sextant init` to populate the home; runtime lookups read the home instead.
pub fn resource_bytes(path: &str) -> Option<Cow<'static, [u8]>> {
    Resources::get(path).map(|f| f.data)
}

/// whether the configuration home has been populated (`sextant init` has run).
/// runtime resource lookups read the home, so an un-initialized home yields no
/// profiles or screen assets.
pub fn is_initialized() -> bool {
    config_home().join("profiles").is_dir()
}

/// bytes of a file under the configuration home, or `None` if absent.
fn home_bytes(rel: &str) -> Option<Cow<'static, [u8]>> {
    std::fs::read(config_home().join(rel)).ok().map(Cow::Owned)
}

/// sorted `.plp` paths of a user profile set on disk (`<home>/profiles/NAME/`),
/// empty when the directory is absent -- the on-disk override the embedded set.
pub fn user_profile_paths(name: &str) -> Vec<PathBuf> {
    let dir = config_home().join("profiles").join(name);
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("plp"))
        .collect();
    paths.sort();
    paths
}

/// sorted names of the user profile sets on disk: subdirectories of
/// `<home>/profiles/` that hold at least one `.plp`.
pub fn user_profile_sets() -> Vec<String> {
    let dir = config_home().join("profiles");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            (!user_profile_paths(&name).is_empty()).then_some(name)
        })
        .collect();
    names.sort();
    names
}

/// sorted names of the profile sets available to `--profiles`, read from the
/// configuration home (`<home>/profiles/`) -- what `sextant profiles` lists.
/// empty until `sextant init` has populated the home.
pub fn available_profile_sets() -> Vec<String> {
    user_profile_sets()
}

/// bytes of a palettegear screen-image asset from the configuration home,
/// preferring `assets/palettegear/N.fmt` over the `.../reference/N.fmt`
/// fallback. the indexed screen images are palettegear's.
pub fn asset_bytes(index: u32, fmt: &str) -> Option<Cow<'static, [u8]>> {
    [
        format!("assets/palettegear/{index}.{fmt}"),
        format!("assets/palettegear/reference/{index}.{fmt}"),
    ]
    .iter()
    .find_map(|p| home_bytes(p))
}

/// the monogram core-screen background from the configuration home: a plain
/// black 240x240 big-endian rgb565 frame (no vendored logo; see
/// docs/monogram-protocol.md).
pub fn monogram_logo() -> Option<Cow<'static, [u8]>> {
    home_bytes("assets/monogram/logo.rgb565")
}

/// the monogram screen font from the configuration home: 95 glyphs (ascii
/// 32..126), 8x8 1bpp, one byte per row (bit 7 leftmost). the monogram has no
/// device-side text rendering, so the host rasterizes status text with this.
pub fn monogram_font() -> Option<Cow<'static, [u8]>> {
    home_bytes("assets/monogram/font8x8.bin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_carries_the_bundled_profiles_and_black_base() {
        // the embedded seed holds the profile sets `init` extracts, plus the
        // black slot-23 base screen; unknown assets are absent.
        assert!(resource_bytes("profiles/multifunction-buttons/00-meta.plp").is_some());
        assert!(resource_bytes("assets/palettegear/23.raw").is_some());
        assert!(resource_bytes("assets/palettegear/999.raw").is_none());
    }

    #[test]
    fn config_home_prefers_absolute_xdg_then_home() {
        let abs = config_home_from(Some("/x/cfg".into()), Some("/home/u".into()));
        assert_eq!(PathBuf::from("/x/cfg/sextant"), abs);
        // a relative XDG value is ignored, falling back to $HOME/.config.
        let rel = config_home_from(Some("cfg".into()), Some("/home/u".into()));
        assert_eq!(PathBuf::from("/home/u/.config/sextant"), rel);
        let neither = config_home_from(None, None);
        assert_eq!(PathBuf::from(".config/sextant"), neither);
    }

    #[test]
    fn resource_paths_mirror_the_embedded_tree() {
        let paths = resource_paths();
        assert!(paths.windows(2).all(|w| w[0] <= w[1]));
        assert!(paths.iter().any(|p| p.starts_with("profiles/")));
        assert!(paths.iter().any(|p| p == "scripts/hello.sh"));
    }
}
