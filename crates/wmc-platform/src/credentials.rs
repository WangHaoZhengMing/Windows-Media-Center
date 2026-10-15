#[cfg(target_os = "windows")]
mod windows {
    use anyhow::{Context, Result};
    use keyring_core::{Entry, Error, api::CredentialStoreApi};
    use std::collections::HashMap;

    fn entry(target: &str, user: &str) -> Result<Entry> {
        let store =
            windows_native_keyring_store::Store::new().context("无法打开 Windows 凭据管理器")?;
        store
            .build(
                "Wmc.MediaCenter.Translation",
                user,
                Some(&HashMap::from([
                    ("target", target),
                    ("persistence", "Local"),
                ])),
            )
            .context("无法创建翻译密钥凭据")
    }

    fn translation_entry(microsoft: bool) -> Result<Entry> {
        let provider = if microsoft {
            "MicrosoftTranslator"
        } else {
            "GoogleCloud"
        };
        entry(&format!("Wmc.MediaCenter/Translation/{provider}"), provider)
    }

    fn read(entry: &Entry) -> Result<Option<String>> {
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(Error::NoEntry) => Ok(None),
            Err(_) => anyhow::bail!("无法读取 Windows 中保存的翻译密钥"),
        }
    }

    fn remove(entry: &Entry) -> Result<()> {
        match entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(_) => anyhow::bail!("无法删除 Windows 中保存的翻译密钥"),
        }
    }

    pub fn load_translation_key(microsoft: bool) -> Result<Option<String>> {
        read(&translation_entry(microsoft)?)
    }

    pub fn save_translation_key(microsoft: bool, key: &str) -> Result<()> {
        anyhow::ensure!(!key.trim().is_empty(), "API Key 不能为空");
        translation_entry(microsoft)?
            .set_password(key)
            .map_err(|_| anyhow::anyhow!("无法将翻译密钥保存到 Windows 凭据管理器"))
    }

    pub fn delete_translation_key(microsoft: bool) -> Result<()> {
        remove(&translation_entry(microsoft)?)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::{SystemTime, UNIX_EPOCH};

        struct TemporaryCredential(Entry);
        impl Drop for TemporaryCredential {
            fn drop(&mut self) {
                let _ = self.0.delete_credential();
            }
        }

        #[test]
        fn native_credentials_survive_new_handles_and_delete_cleanly() {
            let target = format!(
                "Wmc.MediaCenter/Tests/{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let temporary = TemporaryCredential(entry(&target, "test-provider").unwrap());
            assert!(read(&temporary.0).unwrap().is_none());
            temporary.0.set_password("disposable-test-key").unwrap();
            let reopened = entry(&target, "test-provider").unwrap();
            assert_eq!(
                read(&reopened).unwrap().as_deref(),
                Some("disposable-test-key")
            );
            assert_eq!(
                reopened
                    .get_attributes()
                    .unwrap()
                    .get("persistence")
                    .map(String::as_str),
                Some("Local")
            );
            reopened.set_password("updated-test-key").unwrap();
            assert_eq!(
                read(&temporary.0).unwrap().as_deref(),
                Some("updated-test-key")
            );
            remove(&reopened).unwrap();
            assert!(read(&temporary.0).unwrap().is_none());
            remove(&temporary.0).unwrap();
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::{delete_translation_key, load_translation_key, save_translation_key};

#[cfg(not(target_os = "windows"))]
pub fn load_translation_key(_: bool) -> anyhow::Result<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn save_translation_key(_: bool, _: &str) -> anyhow::Result<()> {
    anyhow::bail!("当前平台尚未接入系统凭据存储")
}

#[cfg(not(target_os = "windows"))]
pub fn delete_translation_key(_: bool) -> anyhow::Result<()> {
    anyhow::bail!("当前平台尚未接入系统凭据存储")
}
