use gpui_kit::{AssetSource, SharedString};
use std::borrow::Cow;

/// The executable carries its own artwork; working directory is irrelevant.
pub(crate) struct MediaAssets;

impl AssetSource for MediaAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(wmc_assets::load(path))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(wmc_assets::all()
            .filter(|asset| asset.starts_with(path))
            .map(Into::into)
            .collect())
    }
}
