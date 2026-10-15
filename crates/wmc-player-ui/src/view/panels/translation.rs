use super::{PlayerView, text};
use gpui_kit::{
    base::input::{Input, InputState},
    prelude::*,
    *,
};
use std::{collections::BTreeMap, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
use wmc_application::settings::TranslationPreferences as Preferences;
use wmc_enrichment::translation::{
    Cancellation, Document, GoogleTranslator, MicrosoftTranslator, Progress, TranslationJob,
    Translator,
};

pub(in crate::view) const LYRIC_GAP: f32 = 6.;

struct TranslatedLine {
    rows: Vec<String>,
    font_size: f32,
    line_height: f32,
}

pub(in crate::view) struct TranslationState {
    pub open: bool,
    pub enabled: bool,
    pub rows: BTreeMap<usize, String>,
    wrapped: BTreeMap<usize, TranslatedLine>,
    layout_key: Option<(u32, u32)>,
    microsoft: bool,
    key: Entity<InputState>,
    region: Entity<InputState>,
    source: Entity<InputState>,
    target: Entity<InputState>,
    busy: bool,
    failed: bool,
    message: String,
    generation: u64,
    cancel: Option<Cancellation>,
    task: Option<Task<()>>,
}
impl TranslationState {
    pub fn new(
        app: &wmc_application::Application,
        window: &mut Window,
        cx: &mut Context<PlayerView>,
    ) -> Self {
        let prefs = Preferences::load();
        Self {
            open: false,
            enabled: prefs.enabled,
            rows: BTreeMap::new(),
            wrapped: BTreeMap::new(),
            layout_key: None,
            microsoft: prefs.microsoft,
            key: cx.new(|cx| {
                let mut state = InputState::new(window, cx)
                    .masked(true)
                    .default_value(app.translation_key(prefs.microsoft));
                state.set_editor_style(super::settings::editor_style());
                state
            }),
            region: cx.new(|cx| {
                let mut state = InputState::new(window, cx).default_value(prefs.region);
                state.set_editor_style(super::settings::editor_style());
                state
            }),
            source: cx.new(|cx| {
                let mut state = InputState::new(window, cx).default_value(prefs.source);
                state.set_editor_style(super::settings::editor_style());
                state
            }),
            target: cx.new(|cx| {
                let mut state = InputState::new(window, cx).default_value(prefs.target);
                state.set_editor_style(super::settings::editor_style());
                state
            }),
            busy: false,
            failed: false,
            message: "按批翻译，每批最多 6 行；密钥仅保留在内存中。".into(),
            generation: 0,
            cancel: None,
            task: None,
        }
    }
    pub fn stop(&mut self) {
        self.generation += 1;
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
        }
        self.task = None;
        self.busy = false;
    }
    pub fn invalidate(&mut self) {
        self.stop();
        self.rows.clear();
        self.wrapped.clear();
        self.layout_key = None;
    }
    pub fn prepare_layout(
        &mut self,
        width: f32,
        scale: f32,
        lyrics: &super::lyrics::layout::Layouts,
        window: &Window,
    ) {
        let key = (width.to_bits(), scale.to_bits());
        if self.layout_key == Some(key) {
            return;
        }
        self.wrapped.clear();
        for (&id, translated) in &self.rows {
            let text = translated.split_whitespace().collect::<Vec<_>>().join(" ");
            let font_size = lyrics.lines.get(id).map_or(29., |line| line.font_size) * 0.6;
            let line_height = font_size * 1.25;
            let shaped = text::shape(
                &text,
                font_size * scale,
                FontWeight::BOLD,
                rgb(0xffffff).into(),
                window,
            );
            let mut rows = vec![];
            let mut start = 0;
            while start < text.len() {
                let mut end = start;
                let mut space = None;
                for (offset, grapheme) in text[start..].grapheme_indices(true) {
                    let next = start + offset + grapheme.len();
                    if f32::from(shaped.x_for_index(next) - shaped.x_for_index(start))
                        > width * scale * 0.85
                        && end > start
                    {
                        end = space.unwrap_or(end);
                        break;
                    }
                    end = next;
                    if grapheme.chars().all(char::is_whitespace) {
                        space = Some(end);
                    }
                }
                rows.push(text[start..end].trim().to_owned());
                start = end;
            }
            self.wrapped.insert(
                id,
                TranslatedLine {
                    rows,
                    font_size,
                    line_height,
                },
            );
        }
        self.layout_key = Some(key);
    }
    pub fn height(&self, id: usize) -> f32 {
        if self.enabled {
            self.wrapped.get(&id).map_or(0., |line| {
                line.rows.len() as f32 * line.line_height + LYRIC_GAP
            })
        } else {
            0.
        }
    }
    pub fn line(&self, id: usize, scale: f32) -> Div {
        let Some(layout) = self.wrapped.get(&id).filter(|_| self.enabled) else {
            return div();
        };
        div()
            .font_family("Microsoft YaHei UI")
            .font_weight(FontWeight::BOLD)
            .text_size(px(layout.font_size * scale))
            .line_height(px(layout.line_height * scale))
            .text_color(rgb(0xffffff).opacity(0.65))
            .children(layout.rows.iter().map(|row| div().child(row.clone())))
    }
}
impl Drop for TranslationState {
    fn drop(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }
}
use super::settings_ui::{self, button};
fn field(label: &'static str, state: &Entity<InputState>) -> Div {
    div()
        .mt(px(12.))
        .child(div().mb(px(5.)).text_size(px(13.)).child(label))
        .child(
            div()
                .h(px(36.))
                .px(px(8.))
                .py(px(4.))
                .rounded(px(5.))
                .border_1()
                .border_color(rgb(0x33546c))
                .bg(rgb(0x081b2b))
                .child(Input::new(state)),
        )
}
impl PlayerView {
    fn translation_preferences(&self, cx: &App) -> Preferences {
        Preferences {
            microsoft: self.translation.microsoft,
            source: self.translation.source.read(cx).value().trim().into(),
            target: self.translation.target.read(cx).value().trim().into(),
            region: self.translation.region.read(cx).value().trim().into(),
            enabled: self.translation.enabled,
        }
    }
    fn persist_translation_preferences(&self, cx: &Context<Self>) {
        let prefs = self.translation_preferences(cx);
        cx.background_executor()
            .spawn(async move {
                if let Err(error) = prefs.save() {
                    eprintln!("翻译设置保存失败：{error:#}");
                }
            })
            .detach();
    }
    pub(in crate::view) fn restore_translation_cache(&mut self, cx: &mut Context<Self>) {
        if !self.translation.enabled {
            return;
        }
        let Ok(document) = Document::from_lyrics(&self.lyrics) else {
            return;
        };
        let prefs = self.translation_preferences(cx);
        let generation = self.translation.generation;
        self.translation.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let root = wmc_platform::paths::app_cache_root()?
                        .join("media-cache")
                        .join("translations");
                    TranslationJob::cached_rows(
                        &document,
                        &prefs.provider_id(),
                        &prefs.source,
                        &prefs.target,
                        &root,
                    )
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.translation.generation != generation {
                    return;
                }
                match result {
                    Ok(rows) => {
                        this.translation.rows.extend(rows);
                        this.translation.layout_key = None;
                    }
                    Err(error) => eprintln!("翻译缓存读取失败：{error:#}"),
                }
                cx.notify();
            });
        }));
    }
    pub(in crate::view) fn open_translation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_more = false;
        self.lyrics_search.open = false;
        self.settings.info_open = false;
        if self.cover_search.open {
            self.close_cover_search();
        }
        self.translation.open = true;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn start_translation(&mut self, cx: &mut Context<Self>) {
        self.translation.stop();
        let source = self.translation.source.read(cx).value().trim().to_owned();
        let target = self.translation.target.read(cx).value().trim().to_owned();
        let key = self.translation.key.read(cx).value().trim().to_owned();
        if !key.is_empty() {
            self.app
                .set_translation_key(self.translation.microsoft, key.clone());
        }
        let region = self.translation.region.read(cx).value().trim().to_owned();
        let result = (|| -> anyhow::Result<_> {
            anyhow::ensure!(
                !target.is_empty()
                    && target.len() < 32
                    && target
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-'),
                "请输入有效的目标语言，例如 zh-CN、en、ja"
            );
            let document = Document::from_lyrics(&self.lyrics)?;
            let provider: Arc<dyn Translator> = if self.translation.microsoft {
                Arc::new(MicrosoftTranslator::new(key, region)?)
            } else {
                Arc::new(GoogleTranslator::new(key)?)
            };
            let root = wmc_platform::paths::app_cache_root()?
                .join("media-cache")
                .join("translations");
            Ok((document, provider, root))
        })();
        let (document, provider, root) = match result {
            Ok(result) => result,
            Err(error) => {
                self.translation.message = error.to_string();
                cx.notify();
                return;
            }
        };
        let generation = self.translation.generation;
        self.translation.rows.clear();
        self.translation.layout_key = None;
        self.translation.enabled = true;
        self.translation.busy = true;
        self.translation.failed = false;
        self.translation.message = "正在读取缓存并翻译…".into();
        self.persist_translation_preferences(cx);
        let priority = self.lyrics.active_line(self.app.position()).unwrap_or(0);
        let identity = self.media_identity.clone();
        self.translation.task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let job =
                        TranslationJob::start(document, provider, source, target, root, priority)?;
                    if let Some(identity) = identity
                        && let Err(error) = wmc_enrichment::cache::MediaCache::default_location()
                            .and_then(|cache| cache.record_translation(&identity, &job.cache_file))
                    {
                        eprintln!("译文索引记录失败：{error:#}");
                    }
                    Ok::<_, anyhow::Error>(job)
                })
                .await;
            let job = match result {
                Ok(job) => job,
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        if this.translation.generation == generation {
                            this.translation.busy = false;
                            this.translation.message = error.to_string();
                            cx.notify();
                        }
                    });
                    return;
                }
            };
            if this
                .update(cx, |this, _| {
                    if this.translation.generation != generation {
                        return false;
                    }
                    this.translation.cancel = Some(job.token());
                    true
                })
                .ok()
                != Some(true)
            {
                return;
            }
            while let Ok(progress) = job.events.recv().await {
                let finished = matches!(progress, Progress::Finished);
                if this
                    .update(cx, |this, cx| {
                        if this.translation.generation != generation {
                            return false;
                        }
                        match progress {
                            Progress::Rows(rows) => {
                                this.translation.rows.extend(rows);
                                this.translation.layout_key = None;
                                if !this.translation.failed {
                                    this.translation.message =
                                        format!("已翻译 {} 行", this.translation.rows.len());
                                }
                            }
                            Progress::Error(error) => {
                                this.translation.failed = true;
                                this.translation.message = error;
                            }
                            Progress::Finished => {
                                this.translation.busy = false;
                                if !this.translation.failed {
                                    this.translation.message =
                                        format!("翻译完成 · {} 行", this.translation.rows.len());
                                }
                            }
                        }
                        cx.notify();
                        true
                    })
                    .ok()
                    != Some(true)
                {
                    break;
                }
                if finished {
                    break;
                }
            }
        }));
        cx.notify();
    }
    pub(in crate::view) fn translation_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let form = div()
            .child(
                div()
                    .mt(px(12.))
                    .flex()
                    .flex_wrap()
                    .gap(px(10.))
                    .child(
                        button(
                            "translation-ms",
                            if self.translation.microsoft {
                                "✓ 微软 Translator"
                            } else {
                                "微软 Translator"
                            },
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !this.translation.microsoft {
                                this.translation.stop();
                                this.translation.microsoft = true;
                                this.translation.key.update(cx, |key, cx| {
                                    key.set_value(
                                        this.app.translation_key(this.translation.microsoft),
                                        window,
                                        cx,
                                    )
                                });
                            }
                            cx.notify();
                        })),
                    )
                    .child(
                        button(
                            "translation-google",
                            if self.translation.microsoft {
                                "Google Cloud"
                            } else {
                                "✓ Google Cloud"
                            },
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.translation.microsoft {
                                this.translation.stop();
                                this.translation.microsoft = false;
                                this.translation.key.update(cx, |key, cx| {
                                    key.set_value(
                                        this.app.translation_key(this.translation.microsoft),
                                        window,
                                        cx,
                                    )
                                });
                            }
                            cx.notify();
                        })),
                    ),
            )
            .child(field("API Key（仅保留在内存）", &self.translation.key))
            .when(self.translation.microsoft, |form| {
                form.child(field(
                    "微软资源区域（全局资源可留空）",
                    &self.translation.region,
                ))
            })
            .child(field("源语言（auto 自动识别）", &self.translation.source))
            .child(field(
                "目标语言（zh-CN / zh-TW / en / ja 等）",
                &self.translation.target,
            ))
            .child(
                div()
                    .mt(px(16.))
                    .flex()
                    .flex_wrap()
                    .gap(px(10.))
                    .child(
                        button(
                            "translation-start",
                            if self.translation.busy {
                                "重新开始"
                            } else {
                                "开始 / 继续翻译"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.start_translation(cx))),
                    )
                    .child(button("translation-stop", "取消").on_click(cx.listener(
                        |this, _, _, cx| {
                            this.translation.stop();
                            this.translation.message = "已取消，完成的译文已保留".into();
                            cx.notify();
                        },
                    )))
                    .child(
                        button(
                            "translation-toggle",
                            if self.translation.enabled {
                                "隐藏译文"
                            } else {
                                "显示译文"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.translation.enabled = !this.translation.enabled;
                            this.persist_translation_preferences(cx);
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .mt(px(14.))
                    .text_size(px(13.))
                    .child(self.translation.message.clone()),
            )
            .child(
                div()
                    .mt(px(8.))
                    .text_size(px(12.))
                    .text_color(rgb(0xa8c9df))
                    .child("使用正式云翻译接口，需开通对应服务；点击开始后发送歌词。"),
            );
        let close =
            button("translation-close", "关闭").on_click(cx.listener(|this, _, window, cx| {
                this.translation.open = false;
                window.focus(&this.focus, cx);
                cx.notify();
            }));
        settings_ui::panel("translation-overlay", "歌词翻译", window, close, form, true)
    }
}
