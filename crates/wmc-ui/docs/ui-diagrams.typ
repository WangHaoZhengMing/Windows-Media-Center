// Implementation snapshot: 2026-10-14. Refresh art and playback integration match Rust.
// Shared drawing helpers. Compile previews with the repository as --root.
#import "@preview/cetz:0.5.2": canvas, draw

#let assets = "../../wmc-assets/assets/wmc/"
#let ink = rgb("091735")
#let muted = rgb("7895af")
#let accent = rgb("087dba")
#let ice = rgb("b7e6ff")
#let paper = rgb("f1f5f8")

#let doc-style(body) = {
  set page(width: 297mm, height: 210mm, margin: 13mm,
    footer: context align(right, text(size: 8pt, fill: muted)[WMC · 实现图解 · 2026-10-14 / #counter(page).display("01")]))
  set text(font: ("Microsoft YaHei", "Segoe UI"), size: 9pt, fill: ink, lang: "zh")
  set par(leading: 0.55em, spacing: 0.8mm)
  set heading(numbering: none)
  set table(stroke: 0.4pt + rgb("d6e1e8"), inset: 4pt)
  body
}

#let chapter(kicker, title, subtitle) = [
  #text(size: 8pt, weight: "bold", fill: accent, tracking: 1pt)[#kicker]
  #v(1.5mm)
  #text(size: 21pt, weight: "bold")[#title]
  #v(1.5mm)
  #text(size: 9pt, fill: muted)[#subtitle]
  #v(5mm)
]

#let note(title, body) = block(width: 100%, fill: paper, inset: 8pt, radius: 3pt)[
  #text(weight: "bold", fill: accent)[#title] #h(5pt) #body
]

// All raster UI art is loaded directly from wmc-assets; none is redrawn.
#let raster(x, y, w, h, path, fit: "stretch") = draw.content(
  (x, y), anchor: "north-west",
  image(assets + path, width: w * 1mm, height: h * 1mm, fit: fit),
)

#let label(x, y, body, size: 9pt, fill: ink, anchor: "north-west", width: auto) = {
  draw.content((x, y), anchor: anchor, box(width: width, text(size: size, fill: fill, body)))
}

#let panel(x, y, w, h, fill: paper, stroke: none) = {
  draw.rect((x, y - h), (x + w, y), fill: fill, stroke: stroke)
}

#let control(x, y, w, h, name, state: "NOFOCUS") = {
  raster(x, y, w, h, "controls/TWOFOOT.CONTROL." + name + "." + state + ".PNG", fit: "contain")
}

#let focus-art(x, y, w, name, frame: true) = {
  raster(x, y, w, w * 145 / 258, "shortcuts/STARTMENU.QUICKLINK." + name + ".FOCUS.PNG")
  if frame {
    raster(x, y, w, w * 145 / 258, "glass/STARTMENU.QUICKLINK.BACKGROUND.PNG")
  }
}

#let callout(number, start, end, title, body, width: 48mm) = {
  draw.line(start, (end.at(0) - 4, start.at(1)), (end.at(0) - 4, end.at(1)),
    stroke: 0.65pt + accent)
  draw.circle(start, radius: 2.5, fill: accent, stroke: white)
  label(start.at(0), start.at(1), str(number), size: 8pt, fill: white, anchor: "center")
  label(end.at(0), end.at(1) + 2, [#text(weight: "bold", fill: accent)[#title]\ #text(size: 8pt, fill: muted)[#body]], width: width)
}

#let catalog = (
  (title: "照片", entries: (("照片库", "PICTURES"), ("幻灯片", "PICTURES.PLAYFAVS"), ("拍摄日期", "date"), ("收藏", "PICTURES.PLAYFAVS"))),
  (title: "视频", entries: (("视频库", "VIDEOS"), ("最近播放", "VIDEOS"), ("文件夹", "folder"))),
  (title: "音乐", entries: (("音乐库", "MUSIC"), ("专辑", "MUSIC.PLAYFAVS"), ("播放队列", "queue"))),
  (title: "设置", entries: (("媒体位置", "folder"), ("显示与动效", "display"), ("关于", "LEARNMORE"))),
)

#let glyphs = (date: "BROWSE.CMD.SCHEDULED.FOCUS.PNG", folder: "COMMON.ICON.SUBFOLDER.LARGE.PNG",
  queue: "AUDIO.ADDTOQUEUE.ICON.PNG", display: "OPTIONS.CALIBRATION.ICON.DISPLAY.FLAT.PNG")

#let transport-view(s: 0.5, x: 0, y: 24, names: false) = {
  raster(x, y, 436*s, 48*s, "controls/TWOFOOT.FULLSCREEN.BOTTOMBAR.RIGHT.PNG")
  let buttons = (("GUIDE", "队列", true), ("STOP", "停止", false), ("BACKWARD", "快退", false),
    ("PREVIOUS", "上一首", false), ("PLAY", "播放", false), ("NEXT", "下一首", false),
    ("FORWARD", "快进", false), ("MUTE", "静音", true), ("MINUS", "音量 −", true), ("PLUS", "音量 +", true))
  let cursor = x+38*s
  for (i, button) in buttons.enumerate() {
    if i == 1 or i == 2 or i == 7 {
      raster(cursor, y - 14*s, 2*s, 20*s, "controls/TWOFOOT.CONTROL.DIVIDER.PNG")
      cursor += 8*s
    }
    let w = if button.at(0) == "PLAY" {42} else {34}
    let bh = if button.at(0) == "PLAY" {43} else {31}
    control(cursor, y - (48 - bh)/2*s, w*s, bh*s, button.at(0),
      state: if button.at(2) {"NOFOCUS"} else {"DISABLED"})
    if names { label(cursor+w*s/2, y - 52*s, button.at(1), anchor: "north", size: 7pt) }
    cursor += (w+2)*s
  }
}

#let about-view(s) = {
  let h = 720*s
  label(170*s, h - 157*s, "设置", size: 29*s*1mm, fill: rgb("4b6e8e"))
  label(510*s, h - 157*s, "软件版本", size: 24*s*1mm, fill: white)
  raster(192*s, h - 224*s, 260*s, 43*s, "browse/COMMON.BUTTON.NOFOCUS.FULL.PNG")
  label(210*s, h - 235*s, "完成", size: 19*s*1mm, fill: white)
  control(510*s, h - 211*s, 60*s, 60*s, "MENU")
  draw.content((580*s, h - 224*s), anchor: "north-west",
    text(font: "Segoe UI", weight: "light", size: 38*s*1mm, fill: white)[WMC Media Center])
  let lines = ("Windows", "版本：0.1.0", "Rust 重写版 · GPUI Kit 0.7.1", "界面资源：Windows Media Center")
  for (i, value) in lines.enumerate() {
    label(510*s, h - (293+i*24)*s, value, size: 19*s*1mm, fill: rgb("b8cddd"))
  }
  for (i, value) in ("用 Rust 重建熟悉的客厅媒体体验。", "照片、视频与音乐。").enumerate() {
    label(510*s, h - (410+i*24)*s, value, size: 19*s*1mm, fill: rgb("b8cddd"))
  }
  label(510*s, h - 479*s, "平台：Windows / x86_64", size: 19*s*1mm, fill: rgb("b8cddd"))
  label(510*s, h - 503*s, "可用逻辑处理器：运行时读取", size: 19*s*1mm, fill: rgb("b8cddd"))
}

// Current implementation schematics; titles and file values are illustrative.
#let library-view(s, empty: false, focused: 1) = {
  let h = 720*s
  // Same 44-DIP hit target and padded original orb as destination/library.rs.
  draw.content(((1280 - 234)*s, h - 139*s), anchor: "north-west",
    box(width: 44*s*1mm, height: 44*s*1mm, clip: true)[
      #place(top+left, dx: -46*s*1mm, dy: -16.22*s*1mm,
        image(assets+"icons/STARTMENU.QUICKLINK.SYNC.FOCUS.PNG", width: 136*s*1mm, height: 76.44*s*1mm))
    ])
  let cursor = 188
  for (i, value) in ("专辑", "艺术家", "流派", "歌曲", "播放列表", "作曲家", "年份", "专辑艺术家").enumerate() {
    label(cursor*s, h - 145*s, value, size: (if i == 0 {22} else {18})*s*1mm, fill: rgb("00b6df"))
    cursor += value.clusters().len()*18 + 28
  }
  if empty {
    label(224*s, h - 225*s, "没有找到媒体。请在设置 → 媒体位置添加文件夹。", size: 20*s*1mm, fill: muted)
  } else {
    for (i, title) in ("本地歌曲", "未命名专辑", "示例音轨").enumerate() {
      let scale = if i == focused {1.26} else {1}
      let side = 136*scale
      let x = 224+i*160+(136-side)/2
      let top = 225+(136-side)/2
      raster(x*s, h - top*s, side*s, side*s, "browse/AUDIO.NOCOVER.5.LARGE.PNG", fit: "cover")
      panel(x*s, h - top*s, side*s, side*s, fill: none, stroke: 0.4pt+(if i == focused {ice} else {muted}))
      label((224+i*160)*s, h - (365+if i == focused {18} else {0})*s, title, size: 18*s*1mm, fill: white)
    }
  }
  label(224*s, 110*s, "上一页    下一页    1 / 1", size: 16*s*1mm, fill: muted)
  label(224*s, 67*s, if empty {"0 个文件"} else {"3 个文件 · 1 个扫描目录"}, size: 14*s*1mm, fill: muted)
}

#let context-menu-view(x, y, s, selected: 2) = {
  let labels = ("播放", "添加到播放列表", "详细信息", "从本次列表移除", "小图标视图", "列表视图", "管理媒体库", "设置")
  let h = 478*s
  raster(x, y, 287*s, h, "context-menu/OVERLAYS.OUTLINE.PNG")
  raster(x+5*s, y - 5*s, 277*s, 242*s, "context-menu/OVERLAYS.BACKGROUND.TOP.LIGHT.PNG")
  raster(x+5*s, y - 247*s, 277*s, 226*s, "context-menu/OVERLAYS.BACKGROUND.BOTTOM.DARK.PNG")
  for (i, title) in labels.enumerate() {
    let offset = 14+i*55+if i >= 4 {18} else {0}
    if i == selected { raster(x+14*s, y - offset*s, 250*s, 51*s, "context-menu/COMMON.BUTTON.LEFT.FOCUS.PNG") }
    label(x+30*s, y - (offset+12)*s, title, size: 20*s*1mm, fill: white)
  }
}

#let media-details-view(s) = {
  let h = 720*s
  label(1227*s, h - 46*s, "详细信息", anchor: "north-east", size: 48*s*1mm, fill: rgb("19446d"))
  label(170*s, h - 151*s, "音乐 / 详细信息", size: 22*s*1mm, fill: ice)
  raster(184*s, h - 221*s, 256*s, 256*s, "browse/AUDIO.NOCOVER.5.LARGE.PNG", fit: "cover")
  label(510*s, h - 204*s, "本地歌曲", size: 42*s*1mm, fill: white)
  label(510*s, h - 278*s, "MP3 · 3:45 · 9.5 MB", size: 19*s*1mm, fill: ice)
  for (i, row) in (("艺术家", "未提供"), ("专辑", "未提供"), ("采样率", "44.1 kHz"), ("声道", "立体声"), ("码率", "320 kbps"), ("文件名", "示例音轨.mp3")).enumerate() {
    label(510*s, h - (342+i*36)*s, row.at(0), size: 17*s*1mm, fill: muted)
    label(678*s, h - (342+i*36)*s, row.at(1), size: 20*s*1mm, fill: white)
  }
  for (i, title) in ("播放", "复制文件路径", "返回媒体库").enumerate() {
    if i == 0 { raster(170*s, h - (498+i*53)*s, 276*s, 45*s, "browse/COMMON.BUTTON.LEFT.FOCUS.PNG") }
    label(192*s, h - (509+i*53)*s, title, size: 21*s*1mm, fill: ice)
  }
  label(510*s, 116*s, "文件位置", size: 16*s*1mm, fill: muted)
  label(510*s, 88*s, "音乐目录 / 示例音轨.mp3", size: 17*s*1mm, fill: ice)
}

#let folder-flow() = {
  for (i, item) in (("目录草稿", "添加 / 移出 / 恢复默认"), ("保存配置", "library.json · 原子替换"), ("后台扫描", "取消过期任务 / 类型筛选"), ("媒体卡片", "标签 / 封面 / 每页 24 项")).enumerate() {
    let x = i*68
    panel(x, 35, 62, 28, stroke: 0.5pt+muted)
    label(x+4, 31, item.at(0), size: 10pt, fill: accent)
    label(x+4, 20, item.at(1), size: 8pt, width: 54mm)
    if i < 3 { draw.line((x+62, 21), (x+68, 21), mark: (end: ">"), stroke: 0.7pt+accent) }
  }
}

#let destination-view(s, kind) = {
  let h = 720*s
  if kind == "about" {
    about-view(s)
  } else if kind == "file-details" {
    media-details-view(s)
  } else if kind == "display" {
    label((1280 - 53)*s, h - 46*s, "常规", anchor: "north-east", size: 48*s*1mm, fill: rgb("19446d"))
    label(170*s, h - 157*s, "设置", size: 29*s*1mm, fill: rgb("7895af"))
    label(510*s, h - 157*s, "显示与动效", size: 24*s*1mm, fill: white)
    for (i, value) in ("保存", "取消").enumerate() {
      raster(166*s, h - (224+i*51)*s, 235*s, 43*s,
        if i == 0 {"browse/COMMON.BUTTON.NOFOCUS.FULL.PNG"} else {"browse/COMMON.BUTTON.DORMANT.FULL.PNG"})
      label(184*s, h - (237+i*51)*s, value, size: 19*s*1mm, fill: white)
    }
    raster(510*s, h - 228*s, 32*s, 32*s, "browse/FIRSTRUN.CHECKBOX.CHECKED.NOFOCUS.PNG")
    label(558*s, h - 233*s, "启用背景流光", size: 21*s*1mm, fill: white)
    label(510*s, h - 278*s, "关闭后使用静态蓝色背景。", size: 16*s*1mm, fill: rgb("7895af"))
    panel(510*s, h - 330*s, 665*s, 1*s, fill: rgb("37516c"))
    label(510*s, h - 350*s, "界面动画", size: 19*s*1mm, fill: white)
    label(510*s, h - 391*s, "跟随系统的减少动画设置。", size: 16*s*1mm, fill: rgb("7895af"))
  } else {
    label((1280 - 53)*s, h - 46*s, "音乐库", anchor: "north-east", size: 48*s*1mm, fill: rgb("19446d"))
    library-view(s, empty: kind == "music-empty")

  }
}

// Native design coordinates are converted to millimetres by s.
#let home-view(s: 0.16, selected: 2, selected-entry: 0, details: none) = {
  let h = 720 * s
  panel(0, h, 1280*s, h, fill: ink)
  raster(0, h, 1280*s, h, "backgrounds/COMMON.ANIMATED.BACKGROUND.PNG")
  control(8*s, h - 7*s, 42*s, 42*s, "GOBACK", state: if details == none {"DISABLED"} else {"NOFOCUS"})
  control(60*s, h - 8*s, 40*s, 40*s, "MENU")
  for (i, name) in ("MINIMIZE", "MAXIMIZE", "EXIT").enumerate() {
    control((1171+i*35)*s, h - 4*s, 35*s, 28*s, name)
  }
  if details == none {
    let factor = 238/258
    for (i, category) in catalog.enumerate() {
      let row = if i < selected { -(selected - i)*70*factor }
        else if i > selected { (315+(i - selected - 1)*70)*factor } else { 0 }
      let y = 360+row+if i == selected {0} else {-24*factor}
      if y >= 48 and y+55 <= 720 {
        label((if i == selected {210} else {210+20*factor})*s, h - y*s, category.title,
          size: 44*s*1mm, fill: if i == selected {white} else {rgb("406180")})
      }
    }
    for (i, entry) in catalog.at(selected).entries.enumerate() {
      let x = 210 + (i - selected-entry)*235
      if x >= 0 and x+238 <= 1280 {
        let special = glyphs.keys().contains(entry.at(1))
        if i == selected-entry {
          if special {
            raster((x+78)*s, h - (428+26)*s, 82*s, 82*s, "icons/"+glyphs.at(entry.at(1)), fit: "contain")
          } else { focus-art(x*s, h - 428*s, 238*s, entry.at(1), frame: false) }
        } else {
          if special {
            raster((x+93)*s, h - (428+41)*s, 52*s, 52*s, "icons/"+glyphs.at(entry.at(1)), fit: "contain")
          } else {
            raster((x+44)*s, h - (428+25)*s, 150*s, 84.3*s,
              "shortcuts/STARTMENU.QUICKLINK."+entry.at(1)+".NOFOCUS.PNG")
          }
        }
        label((x+119)*s, h - 582*s, entry.at(0), anchor: "north",
          size: (if i == selected-entry {25} else {18})*s*1mm,
          fill: if i == selected-entry {white} else {rgb("577fa2")})
      }
    }
    raster(210*s, h - 428*s, 238*s, 238*145/258*s, "glass/STARTMENU.QUICKLINK.BACKGROUND.PNG")
  } else {
    destination-view(s, details.kind)
  }
  if details == none or details.kind != "file-details" {
    transport-view(s: s, x: 844*s, y: 48*s)
  }
}
