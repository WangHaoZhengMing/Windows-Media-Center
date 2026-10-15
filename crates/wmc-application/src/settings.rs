use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct TranslationPreferences {
    pub microsoft: bool,
    pub source: String,
    pub target: String,
    pub region: String,
    pub enabled: bool,
}
impl Default for TranslationPreferences {
    fn default() -> Self {
        Self {
            microsoft: true,
            source: "auto".into(),
            target: "zh-CN".into(),
            region: String::new(),
            enabled: false,
        }
    }
}
impl TranslationPreferences {
    pub fn provider_id(&self) -> String {
        if self.microsoft {
            format!("microsoft-v3:{}", self.region.trim())
        } else {
            "google-basic-v2".into()
        }
    }
    pub fn load() -> Self {
        (|| -> anyhow::Result<Self> {
            let path = wmc_platform::paths::app_cache_root()?.join("translation-settings.json");
            if !path.is_file() {
                return Ok(Self::default());
            }
            anyhow::ensure!(std::fs::metadata(&path)?.len() < 16384, "翻译配置过大");
            Ok(serde_json::from_slice(&std::fs::read(path)?)?)
        })()
        .unwrap_or_default()
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let root = wmc_platform::paths::app_cache_root()?;
        std::fs::create_dir_all(&root)?;
        let temporary = root.join(format!("translation-settings.{}.tmp", std::process::id()));
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(temporary, root.join("translation-settings.json"))?;
        Ok(())
    }
}
