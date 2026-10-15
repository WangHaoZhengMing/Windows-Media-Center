use super::track::{PreparedTrack, TrackPresentation};
use async_channel::Receiver;
use std::{
    cell::{Ref, RefCell},
    path::PathBuf,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use wmc_platform::{
    media_session::{self, Command, MediaSession, Metadata, Playback, Status},
    volume,
};
use wmc_playback::{PlaybackSession, PlaybackSnapshot, queue::QueueState};

#[derive(Clone)]
pub struct Application(Rc<RefCell<State>>);
struct State {
    playback: PlaybackSession,
    media: MediaSession,
    commands: Option<Receiver<Command>>,
    volume: volume::Controller,
    volume_updates: Receiver<volume::Update>,
    volume_state: Option<volume::State>,
    volume_error: Option<String>,
    volume_changed: Option<Instant>,
    generation: u64,
    loading: bool,
    finished: bool,
    current: Option<TrackPresentation>,
    media_identity: Option<wmc_enrichment::cache::MediaIdentity>,
    embedding: Option<Embedding>,
    embed_message: Option<String>,
    translation_keys: std::collections::BTreeMap<bool, String>,
}
struct Embedding {
    result: Receiver<Result<(PathBuf, wmc_enrichment::cache::MediaIdentity), String>>,
    snapshot: PlaybackSnapshot,
}
#[derive(Clone, Debug)]
pub struct TrackLoad {
    pub generation: u64,
    pub index: usize,
    pub path: PathBuf,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
impl Application {
    pub fn new() -> Self {
        #[cfg(not(test))]
        let (volume, volume_updates) = volume::Controller::start();
        #[cfg(test)]
        let (volume, volume_updates) = volume::Controller::testing();
        Self(Rc::new(RefCell::new(State {
            playback: PlaybackSession::default(),
            media: MediaSession::default(),
            commands: None,
            volume,
            volume_updates,
            volume_state: None,
            volume_error: None,
            volume_changed: None,
            generation: 0,
            loading: false,
            finished: false,
            current: None,
            media_identity: None,
            embedding: None,
            embed_message: None,
            translation_keys: std::collections::BTreeMap::new(),
        })))
    }
    pub fn queue(&self) -> Ref<'_, QueueState> {
        Ref::map(self.0.borrow(), |s| &s.playback.queue)
    }
    pub fn enqueue(&self, path: PathBuf, title: String) -> usize {
        self.0.borrow_mut().playback.queue.enqueue(path, title)
    }
    pub fn find_or_enqueue(&self, path: PathBuf, title: String) -> usize {
        self.0
            .borrow_mut()
            .playback
            .queue
            .find_or_enqueue(path, title)
    }
    pub fn clear_queue(&self) {
        self.cancel_load();
        self.0.borrow_mut().playback.queue.clear();
    }
    pub fn toggle_shuffle(&self) {
        let mut s = self.0.borrow_mut();
        s.playback.queue.shuffle = !s.playback.queue.shuffle;
    }
    pub fn toggle_repeat(&self) {
        let mut s = self.0.borrow_mut();
        s.playback.queue.repeat = !s.playback.queue.repeat;
    }
    pub fn queue_target(&self, forward: bool) -> Option<usize> {
        let random = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as usize;
        self.0.borrow_mut().playback.queue.target(forward, random)
    }
    pub fn begin_load(&self, index: usize) -> Option<TrackLoad> {
        let mut s = self.0.borrow_mut();
        if s.embedding.is_some() {
            return None;
        }
        let path = s.playback.queue.entries().get(index)?.0.clone();
        s.generation += 1;
        s.loading = true;
        Some(TrackLoad {
            generation: s.generation,
            index,
            path,
        })
    }
    pub fn cancel_load(&self) {
        let mut s = self.0.borrow_mut();
        s.generation += 1;
        s.loading = false;
    }
    pub fn accepts(&self, load: &TrackLoad) -> bool {
        {
            let s = self.0.borrow();
            s.generation == load.generation && s.loading
        }
    }
    pub fn loading(&self) -> bool {
        self.0.borrow().loading
    }
    pub fn fail_load(&self, load: &TrackLoad) {
        if self.accepts(load) {
            self.0.borrow_mut().loading = false;
        }
    }
    pub fn install(&self, load: &TrackLoad, track: &mut PreparedTrack) -> bool {
        self.install_with(load, track, wmc_playback::AudioPlayer::new)
    }
    fn install_with(
        &self,
        load: &TrackLoad,
        track: &mut PreparedTrack,
        engine: impl FnOnce(wmc_playback::AudioData) -> wmc_playback::AudioPlayer,
    ) -> bool {
        if !self.accepts(load) {
            return false;
        }
        let Some(audio) = track.audio.take() else {
            return false;
        };
        let mut s = self.0.borrow_mut();
        s.media.detach();
        s.commands = None;
        s.current = Some(track.presentation());
        s.media_identity = None;
        s.playback.audio = Some(engine(audio));
        s.playback.queue.select(load.index);
        s.loading = false;
        s.finished = false;
        if s.playback
            .audio
            .as_ref()
            .is_some_and(|audio| audio.error.is_none())
        {
            let empty = image::RgbaImage::new(1, 1);
            let cover =
                image::imageops::thumbnail(track.cover.as_ref().unwrap_or(&empty), 256, 256);
            let (sender, receiver) = media_session::channel();
            match MediaSession::attach(
                Metadata {
                    title: &track.title,
                    artist: &track.artist,
                    album: &track.album,
                    cover: &cover,
                },
                sender,
            ) {
                Ok(media) => {
                    s.media = media;
                    s.commands = Some(receiver);
                }
                Err(error) => eprintln!("系统媒体控制不可用：{error}"),
            }
        }
        true
    }
    pub fn translation_key(&self, microsoft: bool) -> String {
        self.0
            .borrow()
            .translation_keys
            .get(&microsoft)
            .cloned()
            .unwrap_or_default()
    }
    pub fn set_translation_key(&self, microsoft: bool, key: String) {
        self.0.borrow_mut().translation_keys.insert(microsoft, key);
    }
    pub fn track_summary(&self) -> Option<(String, String)> {
        self.0
            .borrow()
            .current
            .as_ref()
            .map(|t| (t.title.clone(), t.artist.clone()))
    }
    pub fn media_identity(&self) -> Option<wmc_enrichment::cache::MediaIdentity> {
        self.0.borrow().media_identity.clone()
    }
    pub fn set_media_identity(&self, identity: wmc_enrichment::cache::MediaIdentity) {
        self.0.borrow_mut().media_identity = Some(identity);
    }
    pub fn current_track(&self) -> Option<TrackPresentation> {
        self.0.borrow().current.clone()
    }
    pub fn update_lyrics(&self, document: wmc_lyrics::Lyrics, content: String) {
        if let Some(track) = &mut self.0.borrow_mut().current {
            track.lyrics = document;
            track.lyric_content = content;
        }
    }
    pub fn dispatch(&self, command: wmc_domain::MediaCommand) {
        use wmc_domain::MediaCommand::*;
        match command {
            PlayPause => self.toggle(),
            Stop => self.stop(),
            Rewind => {
                let _ = self.seek(self.position() - 5.);
            }
            FastForward => {
                let _ = self.seek(self.position() + 5.);
            }
            ToggleMute => {
                let _ = self.volume_command(volume::Command::ToggleMute);
            }
            VolumeDown => {
                let _ = self.volume_command(volume::Command::Change(-5));
            }
            VolumeUp => {
                let _ = self.volume_command(volume::Command::Change(5));
            }
            Previous | Next => {}
        }
    }
    pub fn has_track(&self) -> bool {
        self.0.borrow().playback.audio.is_some()
    }
    pub fn is_finished(&self) -> bool {
        self.0
            .borrow()
            .playback
            .audio
            .as_ref()
            .is_some_and(|a| a.is_finished())
    }
    pub fn take_finished(&self) -> bool {
        let mut s = self.0.borrow_mut();
        let finished = s.playback.audio.as_ref().is_some_and(|a| a.is_finished());
        let changed = finished && !s.finished && !s.loading;
        if !finished {
            s.finished = false;
        } else if changed {
            s.finished = true;
        }
        changed
    }
    pub fn poll(&self) {
        let mut s = self.0.borrow_mut();
        while let Ok(update) = s.volume_updates.try_recv() {
            match update.result {
                Ok(state) => {
                    s.volume_state = Some(state);
                    s.volume_error = None;
                }
                Err(error) => {
                    s.volume_state = None;
                    s.volume_error = Some(error);
                }
            }
            if update.interacted {
                s.volume_changed = Some(Instant::now());
            }
        }
        while let Some(command) = s.commands.as_ref().and_then(|r| r.try_recv().ok()) {
            if s.embedding.is_some() {
                continue;
            }
            if let Some(audio) = &mut s.playback.audio {
                match command {
                    Command::Play => audio.play(),
                    Command::Pause => audio.pause(),
                    Command::Stop => audio.stop(),
                    Command::Restart => {
                        let _ = audio.seek(0.);
                    }
                    Command::SeekTo(t) if t.is_finite() => {
                        let _ = audio.seek(t);
                    }
                    Command::SeekBy(t) if t.is_finite() => {
                        let _ = audio.seek(audio.position() + t);
                    }
                    _ => {}
                }
            }
        }
        if let Some(result) = s
            .embedding
            .as_ref()
            .and_then(|job| match job.result.try_recv() {
                Ok(value) => Some(value),
                Err(async_channel::TryRecvError::Closed) => Some(Err("写入任务意外结束".into())),
                Err(_) => None,
            })
        {
            let job = s.embedding.take().expect("pending embedding");
            let resumed = s
                .playback
                .audio
                .as_mut()
                .map(|audio| audio.resume_tag_write(job.snapshot));
            let mut message = match result {
                Ok((backup, identity)) => {
                    s.media_identity = Some(identity);
                    format!("已写入源音乐文件\n备份：{}", backup.display())
                }
                Err(e) => format!("写入失败：{e}"),
            };
            if let Some(Err(error)) = resumed {
                message.push_str(&format!("\n播放恢复失败：{error:#}"));
            }
            s.embed_message = Some(message);
        }
        publish(&mut s);
    }
    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.0
            .borrow()
            .playback
            .audio
            .as_ref()
            .map(|a| a.snapshot())
            .unwrap_or(PlaybackSnapshot {
                seconds: 0.,
                duration: 0.,
                playing: false,
                stopped: true,
            })
    }
    pub fn position(&self) -> f64 {
        self.snapshot().seconds
    }
    pub fn duration(&self) -> f64 {
        self.snapshot().duration
    }
    pub fn playing(&self) -> bool {
        self.snapshot().playing
    }
    pub fn raw_position(&self) -> f64 {
        self.0
            .borrow()
            .playback
            .audio
            .as_ref()
            .map_or(0., |a| a.raw_position())
    }
    pub fn spectrum(&self) -> [f32; 3] {
        self.0
            .borrow()
            .playback
            .audio
            .as_ref()
            .map_or([0.; 3], |a| a.spectrum())
    }
    pub fn error(&self) -> Option<String> {
        self.0
            .borrow()
            .playback
            .audio
            .as_ref()
            .and_then(|a| a.error.clone())
    }
    fn control(&self, f: impl FnOnce(&mut wmc_playback::AudioPlayer)) {
        let mut s = self.0.borrow_mut();
        if s.embedding.is_some() {
            return;
        }
        if let Some(audio) = &mut s.playback.audio {
            f(audio);
        }
        publish(&mut s);
    }
    pub fn play(&self) {
        self.control(|a| a.play());
    }
    pub fn pause(&self) {
        self.control(|a| a.pause());
    }
    pub fn stop(&self) {
        self.cancel_load();
        self.control(|a| a.stop());
    }
    pub fn toggle(&self) {
        self.control(|a| a.toggle());
    }
    pub fn seek(&self, t: f64) -> anyhow::Result<()> {
        anyhow::ensure!(t.is_finite(), "播放位置无效");
        let mut s = self.0.borrow_mut();
        anyhow::ensure!(s.embedding.is_none(), "正在写入标签");
        if let Some(audio) = &mut s.playback.audio {
            audio.seek(t)?;
        }
        publish(&mut s);
        Ok(())
    }
    pub fn volume_state(&self) -> Option<volume::State> {
        self.0.borrow().volume_state
    }
    pub fn volume_error(&self) -> Option<String> {
        self.0.borrow().volume_error.clone()
    }
    pub fn volume_changed(&self) -> Option<Instant> {
        self.0.borrow().volume_changed
    }
    pub fn volume_command(&self, command: volume::Command) -> Result<(), String> {
        let mut s = self.0.borrow_mut();
        let result = s.volume.send(command);
        if let Err(error) = &result {
            s.volume_error = Some(error.clone());
            s.volume_changed = Some(Instant::now());
        }
        result
    }
    pub fn update_cover(&self, title: &str, artist: &str, album: &str, cover: &image::RgbaImage) {
        if let Some(track) = &mut self.0.borrow_mut().current {
            track.cover = Some(cover.clone());
        }
        if let Err(error) = self.0.borrow().media.update_metadata(Metadata {
            title,
            artist,
            album,
            cover,
        }) {
            eprintln!("系统封面更新失败：{error:#}");
        }
    }
    pub fn embedding(&self) -> bool {
        self.0.borrow().embedding.is_some()
    }
    pub fn embed_message(&self) -> Option<String> {
        self.0.borrow().embed_message.clone()
    }
    pub fn embed_assets(
        &self,
        path: PathBuf,
        content: String,
        cover: image::RgbaImage,
        query: wmc_domain::TrackQuery,
    ) -> anyhow::Result<()> {
        let mut s = self.0.borrow_mut();
        anyhow::ensure!(s.embedding.is_none(), "已有写入任务");
        let audio = s
            .playback
            .audio
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("没有播放会话"))?;
        let snapshot = audio.suspend_tag_write();
        let (sender, result) = async_channel::bounded(1);
        let started = std::thread::Builder::new()
            .name("embed-media-assets".into())
            .spawn(move || {
                let result = (|| -> anyhow::Result<_> {
                    let mut png = std::io::Cursor::new(Vec::new());
                    cover.write_to(&mut png, image::ImageFormat::Png)?;
                    let backup =
                        wmc_media::audio::write_tags::write_assets(&path, &content, png.get_ref())?;
                    let identity = wmc_enrichment::cache::MediaIdentity::read(&query, Some(&path))?;
                    Ok((backup, identity))
                })()
                .map_err(|e| format!("{e:#}"));
                let _ = sender.send_blocking(result);
            });
        if let Err(error) = started {
            if let Some(audio) = &mut s.playback.audio {
                audio.resume_tag_write(snapshot)?;
            }
            return Err(error.into());
        }
        s.embedding = Some(Embedding { result, snapshot });
        s.embed_message = None;
        publish(&mut s);
        Ok(())
    }
}
fn publish(state: &mut State) {
    if let Some(audio) = &state.playback.audio {
        let snapshot = audio.snapshot();
        let playback = Playback {
            status: if snapshot.stopped {
                Status::Stopped
            } else if snapshot.playing {
                Status::Playing
            } else {
                Status::Paused
            },
            seconds: snapshot.seconds,
            duration: snapshot.duration,
        };
        let _ = state.media.publish(playback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prepared() -> PreparedTrack {
        PreparedTrack::new(
            wmc_playback::AudioData::from_samples(vec![0.; 16000], 1, 16000).unwrap(),
            wmc_lyrics::Lyrics::default(),
            None,
            "test".into(),
            "artist".into(),
            String::new(),
        )
        .unwrap()
    }
    #[test]
    fn newer_loads_cancelled_loads_and_duplicate_results_cannot_replace_session() {
        let app = Application::new();
        app.enqueue("first.wav".into(), "first".into());
        app.enqueue("second.wav".into(), "second".into());
        let first = app.begin_load(0).unwrap();
        let second = app.begin_load(1).unwrap();
        let mut old = prepared();
        assert!(!app.install_with(&first, &mut old, wmc_playback::AudioPlayer::preview));
        assert!(old.audio.is_some());
        app.cancel_load();
        assert!(!app.accepts(&second));
        assert!(!app.loading());
        let current = app.begin_load(0).unwrap();
        let mut track = prepared();
        assert!(app.install_with(&current, &mut track, wmc_playback::AudioPlayer::preview));
        assert!(!app.install_with(&current, &mut track, wmc_playback::AudioPlayer::preview));
        assert_eq!(app.queue().current(), Some(0));
    }
    #[test]
    fn dropping_a_view_handle_keeps_playback_seek_and_queue_alive() {
        let app = Application::new();
        app.enqueue("song.wav".into(), "song".into());
        let load = app.begin_load(0).unwrap();
        let mut track = prepared();
        assert!(app.install_with(&load, &mut track, wmc_playback::AudioPlayer::preview));
        let view_handle = app.clone();
        view_handle.play();
        drop(view_handle);
        assert!(app.playing());
        assert!(app.has_track());
        app.pause();
        app.seek(0.5).unwrap();
        let reopened = app.clone();
        assert_eq!(reopened.snapshot().seconds, 0.5);
        assert_eq!(reopened.queue().current(), Some(0));
        assert_eq!(reopened.current_track().unwrap().title, "test");
    }
    #[test]
    fn stop_invalidates_in_flight_preparation() {
        let app = Application::new();
        app.enqueue("song.wav".into(), "song".into());
        let load = app.begin_load(0).unwrap();
        app.stop();
        assert!(!app.accepts(&load));
        assert!(!app.loading());
    }
    #[test]
    fn an_embedding_job_resumes_audio_after_the_view_handle_is_dropped() {
        let app = Application::new();
        app.enqueue("song.wav".into(), "song".into());
        let load = app.begin_load(0).unwrap();
        let mut track = prepared();
        assert!(app.install_with(&load, &mut track, wmc_playback::AudioPlayer::preview));
        app.play();
        let view_handle = app.clone();
        let (sender, result) = async_channel::bounded(1);
        {
            let mut state = app.0.borrow_mut();
            let snapshot = state.playback.audio.as_mut().unwrap().suspend_tag_write();
            state.embedding = Some(Embedding { result, snapshot });
        }
        drop(view_handle);
        assert!(
            app.begin_load(0).is_none(),
            "a tag writer must retain exclusive ownership of its source"
        );
        sender.try_send(Err("test write failure".into())).unwrap();
        app.poll();
        assert!(!app.embedding());
        assert!(app.playing());
        assert!(app.embed_message().unwrap().contains("test write failure"));
    }
}
