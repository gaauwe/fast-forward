use std::{fs, path::PathBuf};
use anyhow::anyhow;

use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[exclude = "*.DS_Store"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if Self::get(path).is_some() {
            return Self::get(path)
                .map(|f| Some(f.data))
                .ok_or_else(|| anyhow!("could not find asset at path \"{}\"", path));
        }

        fs::read(path)
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(std::convert::Into::into)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(PathBuf::from("assets").join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(std::convert::Into::into)
    }
}
