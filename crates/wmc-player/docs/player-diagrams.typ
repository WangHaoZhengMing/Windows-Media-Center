// Updated 2026-10-14. Wide-view illustration uses 1280 × 720 DIPs.
// Runtime geometry and text scale are separate; see view/layout.rs.
// WMC raster resources remain in wmc-assets; this crate only references them.
#import "@preview/cetz:0.5.2": canvas, draw
#import "../../wmc-ui/docs/ui-diagrams.typ": doc-style, chapter, note, label, panel, control, raster, ink, muted, accent, ice
#let assets = "../../wmc-assets/assets/wmc/"
#let font-size = 48
#let line-height = 60
#let row-gap = 40
#let cover-size = 378
#let column-gap = 64
#let brown = rgb("39252b")

// Match track/prepare.rs's album crop, then ObjectFit::Cover's centered square crop.
#let album(side) = {
  let unit = side / 82
  box(width: side, height: side, clip: true)[
    #place(top + left, dx: -48 * unit, dy: -26.5 * unit,
      image(assets + "shortcuts/STARTMENU.QUICKLINK.MUSIC.PLAYFAVS.FOCUS.PNG", width: 258 * unit, height: 145 * unit, fit: "stretch"))
  ]
}

#let transport(x, y, s, names: false) = {
  raster(x, y, 436*s, 48*s, "controls/TWOFOOT.FULLSCREEN.BOTTOMBAR.RIGHT.PNG")
  let cursor = x + 38*s
  let buttons = (("GUIDE", "队列"), ("STOP", "停止"), ("BACKWARD", "−5 秒"),
    ("PREVIOUS", "重播"), ("PAUSE", "暂停"), ("NEXT", "禁用"),
    ("FORWARD", "+5 秒"), ("MUTE", "静音"), ("MINUS", "音量 −"), ("PLUS", "音量 +"))
  for (i, button) in buttons.enumerate() {
    if i == 1 or i == 2 or i == 7 {
      raster(cursor, y - 14*s, 2*s, 20*s, "controls/TWOFOOT.CONTROL.DIVIDER.PNG")
      cursor += 8*s
    }
    let w = if i == 4 {42} else {34}
    let h = if i == 4 {43} else {31}
    control(cursor, y - (48 - h)/2*s, w*s, h*s, button.at(0),
      state: if i == 5 {"DISABLED"} else {"NOFOCUS"})
    if names { label(cursor+w*s/2, y - 53*s, button.at(1), size: 7pt, anchor: "north") }
    cursor += (w+2)*s
  }
}

#let progress(x, y, w, s, p: 0.3) = {
  draw.line((x, y), (x+w, y), stroke: 2*s*1mm + rgb("75656c"))
  draw.line((x, y), (x+w*p, y), stroke: 2*s*1mm + ice)
  draw.circle((x+w*p, y), radius: 3*s, fill: white)
  label(x, y - 15*s, "0:14", size: 14*s*1mm, fill: ice)
  label(x+w, y - 15*s, "−0:34", size: 14*s*1mm, fill: ice, anchor: "north-east")
}

// Only a defocus schematic: actual runtime uses Gaussian-blurred glyph masks.
#let lyric(x, y, words, s, opacity: 1, focused: false, haze: false) = {
  let color = ice.transparentize(100% * (1 - opacity))
  if haze {
    for (dx, dy) in ((-1.5, 0), (1.5, 0), (0, -1.5), (0, 1.5)) {
      label(x+dx*s, y+dy*s, words, size: font-size*s*1mm,
        fill: ice.transparentize(90%))
    }
  }
  label(x, y, words, size: font-size*s*1mm, fill: if focused {white} else {color})
}

#let player-view(s: 0.18) = {
  let h = 720*s
  panel(0, h, 1280*s, h, fill: brown)
  control(14*s, h - 8*s, 42*s, 42*s, "GOBACK")
  control(64*s, h - 8*s, 40*s, 40*s, "MENU")
  for (i, name) in ("MINIMIZE", "MAXIMIZE", "EXIT").enumerate() {
    control((1170+i*34)*s, h - 4*s, 32*s, 32*s, name)
  }
  let ui = calc.min(1280/1376, 720/1030)
  let left = 1280/4-cover-size/2
  let top = 720*0.4-cover-size/2
  let right = 1280/2+18
  let info = top+cover-size+720*0.04
  draw.content((left*s, h - top*s), anchor: "north-west", album(cover-size*s*1mm))
  panel(left*s, h - top*s, cover-size*s, cover-size*s, fill: none, stroke: 0.35pt+muted)
  label(left*s, h - info*s, "蓝色时刻", size: 20*ui*s*1mm, fill: ice)
  label(left*s, h - (info+26*ui)*s, "WMC Demo · 合成伴奏", size: 18*ui*s*1mm, fill: ice)
  label((left+cover-size - 24*ui)*s, h - (info+8*ui)*s, "⋯", size: 20*ui*s*1mm, fill: ice)
  progress(left*s, h - (info+56*ui)*s, cover-size*s, ui*s)
  let lines = ("当夜色慢慢落下", "让音乐留在身旁", "窗外的风轻轻经过", "我们听见时间流淌", "把回忆交给旋律", "把远方放在心上", "每一束蓝色的光")
  let anchor = (720 - 52)*0.42-line-height*ui/2
  for (i, words) in lines.enumerate() {
    let d = i - 3
    lyric(right*s, h - (anchor+d*(line-height+row-gap)*ui)*s, words, ui*s,
      opacity: if d == 0 {1} else {0.48}, focused: d == 0, haze: calc.abs(d)>1)
  }
  transport((1280 - 436)*s, 48*s, s)
}

#let flow-box(x, y, title, body, width: 58, height: 27) = {
  panel(x, y, width, height, stroke: 0.5pt + rgb("d2dfe8"))
  label(x+4, y - 3, title, size: 10pt, fill: accent)
  label(x+4, y - 11, body, size: 8pt, fill: ink, width: (width - 8)*1mm)
}


// Preparation is CPU/file work; playback and the window are separate consumers.
#let preparation-flow() = {
  flow-box(0, 44, "PlayerConfig", "显式来源 / 标签 / TTML・LRC", width: 61)
  flow-box(69, 44, "准备任务", "流式 Decoder / 歌词 / 封面", width: 61)
  flow-box(138, 44, "PreparedTrack", "一帧背景 / 音频文件源", width: 61)
  flow-box(207, 44, "PlayerView", "播放 / 单后台任务生成背景", width: 61)
  for x in (61, 130, 199) { draw.line((x, 30), (x+8, 30), mark: (end: ">"), stroke: 0.8pt+accent) }
  label(0, 0, "prepare_with_timings 分段计时；不包含设备初始化、窗口调度或首帧呈现。", size: 9pt, fill: muted)
}

#let glyph-cache-flow() = {
  flow-box(0, 69, "SongFont / ab_glyph", "本机字体表 + 歌曲字符集合", width: 81)
  flow-box(94, 69, "按需字形掩码", "重复字符复用 / resize 替换", width: 81)
  flow-box(188, 69, "词光照 / 行离焦", "后台生成纹理 / 固定 σ 级别", width: 81)
  draw.line((81, 55), (94, 55), mark: (end: ">"), stroke: 0.8pt+accent)
  draw.line((175, 55), (188, 55), mark: (end: ">"), stroke: 0.8pt+accent)
  flow-box(94, 27, "view/lyrics/line.rs", "GPU 裁切 / 羽化 / 位置 / alpha", width: 81)
  draw.line((228, 42), (228, 13), (175, 13), mark: (end: ">"), stroke: 0.8pt+accent)
  label(0, 20, [*生命周期*\ 缓存归当前歌曲视图所有\ 退出后不保留全局 Font], size: 9pt, width: 83mm)
}
