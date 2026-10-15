//! WMC song settings, editable source lyrics and explicit embedding into music.
use super::{
    PlayerView, lyrics,
    settings_ui::{self, button, card, label},
};
use gpui_kit::{
    base::input::{Input, InputEditorStyle, InputState, Textarea, TextareaState},
    prelude::*,
    *,
};
use std::path::PathBuf;
use wmc_enrichment::lyrics::{Candidate, DownloadedLyrics, Provider, TrackQuery};

pub(in crate::view) struct Settings {
    title: Entity<InputState>,
    artist: Entity<InputState>,
    album: Entity<InputState>,
    editor: Entity<TextareaState>,
    source_path: Option<PathBuf>,
    pub content: String,
    pub document: Option<DownloadedLyrics>,
    pub info_open: bool,
    pub disable_syllables: bool,
    editing: bool,
    pub embedding: bool,
    message: String,
    message_generation: u64,
    details: Option<usize>,
    save_task: Option<Task<()>>,
}
impl Settings {
    pub(in crate::view) fn set_message(&mut self, message: String) {
        self.message = message;
    }
    pub fn set_document(&mut self, document: &DownloadedLyrics, cx: &mut Context<PlayerView>) {
        self.content = document.content.clone();
        self.document = Some(document.clone());
        self.editing = false;
        self.editor
            .update(cx, |editor, cx| editor.set_readonly(true, cx));
    }
    pub fn new(
        query: &TrackQuery,
        content: String,
        source_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<PlayerView>,
    ) -> Self {
        Self {
            title: cx.new(|cx| {
                let mut s = InputState::new(window, cx).default_value(query.title.clone());
                s.set_editor_style(editor_style());
                s
            }),
            artist: cx.new(|cx| {
                let mut s = InputState::new(window, cx).default_value(query.artist.clone());
                s.set_editor_style(editor_style());
                s
            }),
            album: cx.new(|cx| {
                let mut s = InputState::new(window, cx).default_value(query.album.clone());
                s.set_editor_style(editor_style());
                s
            }),
            editor: cx.new(|cx| {
                let mut s = TextareaState::new(window, cx)
                    .rows(10)
                    .default_value(content.clone());
                s.set_editor_style(editor_style());
                s.set_readonly(true, cx);
                s
            }),
            source_path,
            content,
            document: None,
            info_open: false,
            disable_syllables: false,
            editing: false,
            embedding: false,
            message: String::new(),
            message_generation: 0,
            details: None,
            save_task: None,
        }
    }
}
pub(in crate::view) fn editor_style() -> InputEditorStyle {
    InputEditorStyle {
        foreground: rgb(0xeaf6ff).into(),
        muted_foreground: rgb(0xa6bed0).into(),
        background: rgb(0x081b2b).into(),
        caret: rgb(0xd8f4ff).into(),
        selection: rgb(0x3988ba).opacity(0.45).into(),
        ..Default::default()
    }
}
fn field(label: &'static str, state: &Entity<InputState>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .py(px(4.))
        .child(
            div()
                .w(px(42.))
                .text_size(px(13.))
                .text_color(rgb(0xa6c2d6))
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .h(px(36.))
                .px(px(8.))
                .py(px(5.))
                .overflow_hidden()
                .rounded(px(5.))
                .border_1()
                .border_color(rgb(0x33546c))
                .bg(rgb(0x081b2b))
                .text_color(rgb(0xf0f8ff))
                .child(Input::new(state)),
        )
}
impl PlayerView {
    fn settings_shell(
        &self,
        id: &'static str,
        title: &'static str,
        window: &Window,
        cx: &mut Context<Self>,
        content: Div,
    ) -> Stateful<Div> {
        let close =
            button("close-song-settings", "关闭").on_click(cx.listener(|this, _, window, cx| {
                this.settings.info_open = false;
                this.lyrics_search.open = false;
                this.close_cover_search();
                window.focus(&this.focus, cx);
                cx.notify();
            }));
        settings_ui::panel(id, title, window, close, content, id == "lyric-info")
    }
    fn current_card(&self) -> Div {
        card()
            .flex_shrink_0()
            .flex()
            .gap(px(16.))
            .items_center()
            .child(
                img(self.cover.clone())
                    .size(px(48.))
                    .flex_shrink_0()
                    .object_fit(ObjectFit::Contain),
            )
            .child(
                div()
                    .flex_1()
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight::LIGHT)
                            .child(self.title.clone()),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .text_color(rgb(0xb6d3e5))
                            .text_size(px(13.))
                            .child(self.artist.clone()),
                    )
                    .child(
                        div()
                            .mt(px(4.))
                            .text_size(px(12.))
                            .text_color(rgb(0x8daec5))
                            .child(self.online_query.album.clone()),
                    ),
            )
    }
    fn search_fields(&self, cover: bool, cx: &mut Context<Self>) -> Div {
        card()
            .flex_shrink_0()
            .flex()
            .items_end()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(field("歌手", &self.settings.artist))
                    .child(field("歌名", &self.settings.title))
                    .when(cover, |inputs| {
                        inputs.child(field("专辑", &self.settings.album))
                    }),
            )
            .child(
                button(
                    "search-song-settings",
                    if self.lyrics_search.busy || self.cover_search.busy {
                        "搜索中…"
                    } else {
                        "搜索"
                    },
                )
                .mb(px(4.))
                .bg(rgb(0x17628b))
                .border_color(rgb(0x4388ad))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.lyrics_search.busy || this.cover_search.busy {
                        return;
                    }
                    this.online_query.title = this.settings.title.read(cx).value().to_string();
                    this.online_query.artist = this.settings.artist.read(cx).value().to_string();
                    this.online_query.album = this.settings.album.read(cx).value().to_string();
                    if cover {
                        this.search_online_cover(cx);
                    } else {
                        this.search_online_lyrics(cx);
                    }
                })),
            )
    }
    pub(in crate::view) fn online_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let mut results = div().flex().flex_col().gap(px(6.));
        for (index, candidate) in self.lyrics_search.candidates.iter().enumerate() {
            let chosen = candidate.clone();
            results = results.child(
                div()
                    .id(("lyric-candidate", index))
                    .p(px(12.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(rgb(0x25475e))
                    .bg(rgb(0x102c41))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x245574)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .justify_between()
                            .child(
                                div()
                                    .flex_1()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_size(px(17.))
                                            .child(candidate.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .mt(px(4.))
                                            .text_color(rgb(0xadc9dc))
                                            .child(candidate.artists.join(" / ")),
                                    )
                                    .child(
                                        div()
                                            .mt(px(4.))
                                            .text_size(px(12.))
                                            .text_color(rgb(0x8faec4))
                                            .child(candidate.album.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .px(px(8.))
                                    .py(px(4.))
                                    .rounded(px(4.))
                                    .bg(rgb(0x1c405a))
                                    .text_size(px(11.))
                                    .text_color(rgb(0xb6d9ed))
                                    .child(format!(
                                        "{} · {}",
                                        candidate.provider.name(),
                                        candidate.extension()
                                    )),
                            )
                            .child(
                                div()
                                    .id(("lyric-result-info", index))
                                    .px(px(8.))
                                    .py(px(10.))
                                    .child("ⓘ")
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.settings.details =
                                            if this.settings.details == Some(index) {
                                                None
                                            } else {
                                                Some(index)
                                            };
                                        cx.notify();
                                    })),
                            ),
                    )
                    .when(self.settings.details == Some(index), |row| {
                        row.child(
                            div()
                                .mt(px(8.))
                                .text_size(px(12.))
                                .text_color(rgb(0xb3d4e9))
                                .child(format!(
                                    "编号：{} · 时长：{} 秒",
                                    candidate.id,
                                    candidate.duration_ms.unwrap_or(0) / 1000
                                )),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.lyrics_search.busy {
                            this.select_online_lyrics(chosen.clone(), cx);
                        }
                    })),
            );
        }
        let content = div()
            .h_full()
            .flex()
            .flex_col()
            .min_h_0()
            .when(self.lyrics_search.candidates.is_empty(), |content| {
                content.child(self.current_card()).child(
                    card()
                        .flex_shrink_0()
                        .mt(px(8.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div().child("逐字高亮").child(
                                div()
                                    .mt(px(3.))
                                    .text_size(px(12.))
                                    .text_color(rgb(0xa6c2d6))
                                    .child("按演唱进度逐字点亮歌词"),
                            ),
                        )
                        .child(
                            button(
                                "toggle-song-syllables",
                                if self.settings.disable_syllables {
                                    "关闭"
                                } else {
                                    "开启"
                                },
                            )
                            .bg(rgb(if self.settings.disable_syllables {
                                0x1c405a
                            } else {
                                0x17628b
                            }))
                            .border_color(rgb(if self.settings.disable_syllables {
                                0x3a607b
                            } else {
                                0x66b4d8
                            }))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings.disable_syllables = !this.settings.disable_syllables;
                                this.layouts = lyrics::layout::LayoutCache::default();
                                cx.notify();
                            })),
                        ),
                )
            })
            .child(label("搜索条件"))
            .child(self.search_fields(false, cx))
            .child(label("搜索结果"))
            .child(div().text_size(px(12.)).text_color(rgb(0xa6c2d6)).child(
                if self.lyrics_search.message.is_empty() {
                    "输入歌手和歌名，查找逐字歌词".to_owned()
                } else {
                    self.lyrics_search.message.clone()
                },
            ))
            .child(
                div()
                    .id("settings-result-list")
                    .mt(px(8.))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(results),
            );
        self.settings_shell("lyrics-settings", "歌词设置", window, cx, content)
    }
    pub(in crate::view) fn cover_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let mut results = div().flex().flex_col().gap(px(6.));
        for (index, (candidate, preview)) in self.cover_search.candidates.iter().enumerate() {
            let chosen = candidate.clone();
            let mut art = div().size(px(48.)).flex_shrink_0().bg(rgb(0x071d31));
            if let Some(preview) = preview {
                art = art.child(
                    img(preview.clone())
                        .size_full()
                        .object_fit(ObjectFit::Contain),
                );
            }
            results = results.child(
                div()
                    .id(("cover-candidate", index))
                    .p(px(12.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(rgb(0x25475e))
                    .bg(rgb(0x102c41))
                    .flex()
                    .gap(px(14.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x245574)))
                    .child(art)
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(17.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(candidate.album.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(5.))
                                    .text_color(rgb(0xadc9dc))
                                    .child(candidate.artist.clone()),
                            )
                            .child(
                                div()
                                    .mt(px(5.))
                                    .text_size(px(12.))
                                    .text_color(rgb(0x8faec4))
                                    .child(format!(
                                        "{} · {}",
                                        candidate.source, candidate.release_date
                                    )),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.cover_search.busy {
                            this.select_online_cover(chosen.clone(), cx);
                        }
                    })),
            );
        }
        let content = div()
            .h_full()
            .flex()
            .flex_col()
            .min_h_0()
            .when(self.cover_search.candidates.is_empty(), |content| {
                content.child(self.current_card())
            })
            .child(label("搜索条件"))
            .child(self.search_fields(true, cx))
            .child(label("搜索结果"))
            .child(div().text_size(px(12.)).text_color(rgb(0xa6c2d6)).child(
                if self.cover_search.message.is_empty() {
                    "输入歌曲信息，查找专辑封面".to_owned()
                } else {
                    self.cover_search.message.clone()
                },
            ))
            .child(
                div()
                    .id("settings-result-list")
                    .mt(px(8.))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(results),
            );
        self.settings_shell("cover-settings", "封面设置", window, cx, content)
    }
    pub(in crate::view) fn open_lyric_info(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_more = false;
        self.lyrics_search.open = false;
        self.close_cover_search();
        self.settings.info_open = true;
        if !self.settings.editing {
            self.settings.editor.update(cx, |editor, cx| {
                editor.set_value(self.settings.content.clone(), window, cx);
                editor.set_readonly(true, cx);
            });
        }
        if !self.settings.embedding {
            self.settings.message.clear();
        }
        cx.notify();
    }
    fn commit_lyrics_editor(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.settings.editing {
            return true;
        }
        let content = self.settings.editor.read(cx).value().to_string();
        let parsed = match wmc_lyrics::Lyrics::parse_text(&content, self.app.duration()) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.settings.message = format!("歌词格式错误：{error:#}");
                cx.notify();
                return false;
            }
        };
        if parsed.lines.is_empty() && parsed.plain_text.is_none() {
            self.settings.message = "请输入歌词内容".into();
            cx.notify();
            return false;
        }
        self.lyrics_search.generation += 1;
        self.lyrics_search.task = None;
        let format = if let Some(format) = wmc_lyrics::Lyrics::word_format(&content) {
            format
        } else if content.trim_start().starts_with('<') {
            "TTML"
        } else if parsed.lines.iter().any(|line| line.word_synced) {
            "ELRC"
        } else if !parsed.lines.is_empty() {
            "LRC"
        } else {
            "文本"
        };
        let document = DownloadedLyrics {
            candidate: Candidate {
                provider: Provider::Local,
                id: "edited".into(),
                mid: None,
                title: self.title.clone(),
                artists: vec![self.artist.clone()],
                album: self.online_query.album.clone(),
                duration_ms: Some((self.app.duration() * 1000.) as u64),
                title_aliases: vec![],
                score: 0,
            },
            content,
            format: format.into(),
            source_url: String::new(),
            fetched_at_unix: 0,
            translation: None,
        };
        self.apply_online_lyrics(document.clone(), parsed, cx);
        self.settings.editing = false;
        self.settings
            .editor
            .update(cx, |editor, cx| editor.set_readonly(true, cx));
        self.settings.message = "歌词已应用，正在保存本地缓存…".into();
        self.settings.message_generation += 1;
        let message_generation = self.settings.message_generation;
        let identity = self.media_identity.clone();
        let query = self.online_query.clone();
        let path = self.settings.source_path.clone();
        let generation = self.lyrics_search.generation;
        self.settings.save_task = Some(cx.spawn(async move |this, cx| {
            let saved = cx
                .background_executor()
                .spawn(async move {
                    let identity = identity.map(Ok).unwrap_or_else(|| {
                        wmc_enrichment::cache::MediaIdentity::read(&query, path.as_deref())
                    })?;
                    wmc_enrichment::cache::MediaCache::default_location()?
                        .save_lyrics(&identity, &document)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.lyrics_search.generation == generation
                    && this.settings.message_generation == message_generation
                {
                    this.settings.message = match saved {
                        Ok(_) => "歌词已保存并应用".into(),
                        Err(error) => format!("已应用，缓存保存失败：{error:#}"),
                    };
                    cx.notify();
                }
            });
        }));
        true
    }
    fn embed_current_assets(&mut self, cx: &mut Context<Self>) {
        if self.settings.embedding || !self.commit_lyrics_editor(cx) {
            return;
        }
        let Some(path) = self.settings.source_path.clone() else {
            self.settings.message = "演示曲目没有源音乐文件".into();
            cx.notify();
            return;
        };
        let content = self.settings.content.clone();
        let cover = self.cover.clone();
        let query = self.online_query.clone();
        let result = (|| -> anyhow::Result<()> {
            use anyhow::Context;
            let dimensions = cover.size(0);
            let mut pixels = cover.as_bytes(0).context("封面像素不可用")?.to_vec();
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
            let image = image::RgbaImage::from_raw(
                dimensions.width.0 as u32,
                dimensions.height.0 as u32,
                pixels,
            )
            .context("封面尺寸无效")?;
            self.app.embed_assets(path, content, image, query)
        })();
        match result {
            Ok(()) => {
                self.settings.embedding = true;
                self.settings.message = "正在写入歌词和封面…".into();
            }
            Err(error) => self.settings.message = format!("写入失败：{error:#}"),
        }

        cx.notify();
    }
    pub(in crate::view) fn lyric_info_panel(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let source = self
            .lyrics_search
            .source
            .clone()
            .unwrap_or_else(|| "本地歌词".into());
        let words = self
            .lyrics
            .lines
            .iter()
            .map(|line| line.words.len())
            .sum::<usize>();
        let content = div()
            .child(self.current_card())
            .child(
                card()
                    .child(source)
                    .child(
                        div()
                            .mt(px(6.))
                            .text_size(px(12.))
                            .text_color(rgb(0xa6c2d6))
                            .child(format!(
                                "{} 行 · {} 个词 · {:.1} 秒",
                                self.lyrics.lines.len(),
                                words,
                                self.app.duration()
                            )),
                    )
                    .when_some(self.settings.document.as_ref(), |card, doc| {
                        card.child(
                            div()
                                .mt(px(5.))
                                .text_size(px(12.))
                                .child(format!("格式：{} · {}", doc.format, doc.source_url)),
                        )
                    })
                    .when_some(self.settings.source_path.as_ref(), |card, path| {
                        card.child(
                            div()
                                .mt(px(5.))
                                .text_size(px(12.))
                                .child(path.display().to_string()),
                        )
                    }),
            )
            .child(label("歌词原文 · 保留时间戳与逐字信息"))
            .child(
                div()
                    .h(px(260.))
                    .p(px(8.))
                    .rounded(px(7.))
                    .border_1()
                    .border_color(rgb(0x33546c))
                    .bg(rgb(0x081b2b))
                    .text_color(rgb(0xeaf6ff))
                    .text_size(px(14.))
                    .child(Textarea::new(&self.settings.editor)),
            )
            .child(
                div()
                    .mt(px(14.))
                    .flex()
                    .flex_wrap()
                    .gap(px(10.))
                    .child(
                        button(
                            "edit-lyrics",
                            if self.settings.editing {
                                "保存并应用"
                            } else {
                                "编辑歌词"
                            },
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            if !this.settings.embedding {
                                if this.settings.editing {
                                    this.commit_lyrics_editor(cx);
                                } else {
                                    this.settings.editing = true;
                                    this.settings
                                        .editor
                                        .update(cx, |editor, cx| editor.set_readonly(false, cx));
                                    let focus = this.settings.editor.read(cx).focus_handle(cx);
                                    window.focus(&focus, cx);
                                    cx.notify();
                                }
                            }
                        })),
                    )
                    .when(self.settings.editing, |row| {
                        row.child(
                            button("cancel-lyrics-edit", "取消编辑").on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.settings.editing = false;
                                    this.settings.editor.update(cx, |editor, cx| {
                                        editor.set_value(this.settings.content.clone(), window, cx);
                                        editor.set_readonly(true, cx);
                                    });
                                    cx.notify();
                                },
                            )),
                        )
                    })
                    .child(
                        button(
                            "embed-song-assets",
                            if self.settings.embedding {
                                "正在写入…"
                            } else {
                                "写入歌词和封面"
                            },
                        )
                        .opacity(if self.settings.source_path.is_some() {
                            1.
                        } else {
                            0.45
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.embed_current_assets(cx))),
                    ),
            )
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(12.))
                    .text_color(rgb(0xa6c2d6))
                    .child(if self.settings.source_path.is_some() {
                        "写入源音乐文件前会保留 .wmc-backup 备份"
                    } else {
                        "演示曲目：可编辑歌词，没有可写入的源文件"
                    }),
            )
            .child(
                div()
                    .mt(px(10.))
                    .text_size(px(13.))
                    .text_color(rgb(0xc4e8fa))
                    .child(self.settings.message.clone()),
            );
        self.settings_shell("lyric-info", "歌词信息", window, cx, content)
    }
}
