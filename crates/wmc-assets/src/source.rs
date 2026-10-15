use crate::catalog::ASSETS;
use std::borrow::Cow;

pub fn load(path: &str) -> Option<Cow<'static, [u8]>> {
    ASSETS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, data)| Cow::Borrowed(*data))
}

pub fn all() -> impl Iterator<Item = &'static str> {
    ASSETS.iter().map(|(name, _)| *name)
}
