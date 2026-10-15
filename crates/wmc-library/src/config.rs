use crate::MediaKind;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryConfig {
    pub version: u32,
    pub photos: Vec<PathBuf>,
    pub music: Vec<PathBuf>,
    pub videos: Vec<PathBuf>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            version: 1,
            photos: dirs::picture_dir().into_iter().collect(),
            music: dirs::audio_dir().into_iter().collect(),
            videos: dirs::video_dir().into_iter().collect(),
        }
    }
}

impl LibraryConfig {
    pub fn location() -> Result<PathBuf> {
        Ok(dirs::data_local_dir()
            .context("无法找到本地配置目录")?
            .join("WMC/library.json"))
    }
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let config: Self =
            serde_json::from_slice(&std::fs::read(path)?).context("媒体目录配置无法解析")?;
        ensure!(config.version == 1, "不支持的媒体目录配置版本");
        Ok(config)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("配置路径缺少父目录")?;
        std::fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(&serde_json::to_vec_pretty(self)?)?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|error| error.error)?;
        Ok(())
    }
    pub fn roots(&self, kind: MediaKind) -> &[PathBuf] {
        match kind {
            MediaKind::Photos => &self.photos,
            MediaKind::Music => &self.music,
            MediaKind::Videos => &self.videos,
        }
    }
    pub fn roots_mut(&mut self, kind: MediaKind) -> &mut Vec<PathBuf> {
        match kind {
            MediaKind::Photos => &mut self.photos,
            MediaKind::Music => &mut self.music,
            MediaKind::Videos => &mut self.videos,
        }
    }
    pub fn add(&mut self, kind: MediaKind, path: PathBuf) -> Result<()> {
        ensure!(path.is_dir(), "请选择存在的文件夹");
        let canonical = std::fs::canonicalize(&path)?;
        if !self
            .roots(kind)
            .iter()
            .any(|root| std::fs::canonicalize(root).ok().as_ref() == Some(&canonical))
        {
            self.roots_mut(kind).push(path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_empty_roots_stay_empty_and_replacement_is_supported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        let mut config = LibraryConfig {
            version: 1,
            photos: vec![],
            music: vec![],
            videos: vec![],
        };
        config.save(&path).unwrap();
        assert!(LibraryConfig::load(&path).unwrap().music.is_empty());
        config.add(MediaKind::Music, dir.path().to_owned()).unwrap();
        config.add(MediaKind::Music, dir.path().join(".")).unwrap();
        assert_eq!(config.music.len(), 1);
        config.save(&path).unwrap();
        assert_eq!(LibraryConfig::load(&path).unwrap().music.len(), 1);
    }
    #[test]
    fn broken_config_is_reported_without_overwriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        std::fs::write(&path, "broken").unwrap();
        assert!(LibraryConfig::load(&path).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "broken");
    }
}
