use super::PlayerView;
use gpui_kit::*;
use wmc_enrichment::cache::{MediaCache, MediaIdentity};
impl PlayerView {
    pub(super) fn resolve_cached_assets(
        &mut self,
        path: Option<std::path::PathBuf>,
        lyrics_allowed: bool,
        cover_allowed: bool,
        cover_network_needed: bool,
        local_sources: wmc_enrichment::cache::LocalSources,
        cx: &mut Context<Self>,
    ) {
        let query = self.online_query.clone();
        self._asset_task = Some(cx.spawn(async move |this, cx| {
            let identity = cx
                .background_executor()
                .spawn(async move {
                    let identity =
                        MediaIdentity::read(&query, path.as_deref()).unwrap_or_else(|error| {
                            eprintln!("音频 MD5 读取失败，使用歌名和艺术家：{error:#}");
                            MediaIdentity::read(&query, None).expect("metadata identity")
                        });
                    if let Err(error) = MediaCache::default_location()
                        .and_then(|cache| cache.record_track_with_local(&identity, local_sources))
                    {
                        eprintln!("缓存索引记录失败：{error:#}");
                    }
                    identity
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.app.set_media_identity(identity.clone());
                this.media_identity = Some(identity);
                if lyrics_allowed && this.lyrics_search.generation == 0 {
                    let missing = this.lyrics.lines.is_empty() && this.lyrics.plain_text.is_none();
                    this.load_cached_lyrics(missing, cx);
                }
                if cover_allowed && this.cover_search.generation == 0 {
                    this.load_cached_cover(cover_network_needed, cx);
                }
            });
        }));
    }
}
