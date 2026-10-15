use crate::audio::properties::AudioProperties;
use std::path::Path;
use wmc_domain::MediaKind;
#[derive(Default)]
pub struct FileFacts {
    pub size: Option<u64>,
    pub modified: Option<std::time::SystemTime>,
    pub dimensions: Option<(u32, u32)>,
    pub audio: Option<AudioProperties>,
    pub available: bool,
    pub warning: Option<String>,
}

impl FileFacts {
    pub fn read(path: &Path, kind: MediaKind) -> Self {
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => {
                return Self {
                    warning: Some("此路径已不是媒体文件。".into()),
                    ..Default::default()
                };
            }
            Err(error) => {
                return Self {
                    warning: Some(format!("无法访问此文件：{error}")),
                    ..Default::default()
                };
            }
        };
        let mut facts = Self {
            size: Some(metadata.len()),
            modified: metadata.modified().ok(),
            available: true,
            ..Default::default()
        };
        match kind {
            MediaKind::Music => match AudioProperties::read(path) {
                Ok(audio) => facts.audio = Some(audio),
                Err(_) => facts.warning = Some("音频属性暂时无法读取，文件信息仍可查看。".into()),
            },
            MediaKind::Photos => match image::image_dimensions(path) {
                Ok(dimensions) => facts.dimensions = Some(dimensions),
                Err(_) => facts.warning = Some("图片尺寸暂时无法读取，文件信息仍可查看。".into()),
            },
            MediaKind::Videos => {}
        }
        facts
    }
}
