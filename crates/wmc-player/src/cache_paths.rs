use anyhow::{Context, Result};
use std::path::PathBuf;

pub(crate) fn app_cache_root() -> Result<PathBuf> {
    root_for(
        std::env::consts::OS,
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}
fn root_for(
    os: &str,
    local: Option<PathBuf>,
    xdg: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf> {
    let root = match os {
        "windows" => local,
        "macos" => home.map(|home| home.join("Library").join("Caches")),
        _ => xdg
            .filter(|path| path.is_absolute())
            .or_else(|| home.map(|home| home.join(".cache"))),
    }
    .context("找不到系统缓存目录")?;
    Ok(root.join("Windows-Media-Center"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_platform_uses_its_native_cache_directory() {
        assert_eq!(
            root_for("windows", Some("local".into()), None, Some("home".into())).unwrap(),
            PathBuf::from("local/Windows-Media-Center")
        );
        assert_eq!(
            root_for("macos", None, None, Some("home".into())).unwrap(),
            PathBuf::from("home/Library/Caches/Windows-Media-Center")
        );
        let absolute = std::env::temp_dir().join("xdg-cache");
        assert_eq!(
            root_for("linux", None, Some(absolute.clone()), Some("home".into())).unwrap(),
            absolute.join("Windows-Media-Center")
        );
        assert_eq!(
            root_for("linux", None, Some("relative".into()), Some("home".into())).unwrap(),
            PathBuf::from("home/.cache/Windows-Media-Center")
        );
    }
}
