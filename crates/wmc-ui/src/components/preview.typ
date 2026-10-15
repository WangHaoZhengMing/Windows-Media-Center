// From the repository root:
// typst compile --root . crates/wmc-ui/src/components/preview.typ target/ui-docs/components.pdf
#import "../../docs/ui-diagrams.typ": *
#show: doc-style

#note("核对日期 · 2026-10-14", [资源路径与当前清单对应；补充了同步刷新圆球、播放器九宫格菜单和本地缓存资源的边界。])

#chapter("COMPONENTS / 01", "背景与缩略图：资源进入布局", "background.rs 和 artwork.rs 分开绘制。图中位图均直接引用 assets/wmc；边框、标签和箭头由 CeTZ 绘制。")

#canvas(length: 1mm, {
  raster(0, 81, 130, 73.125, "backgrounds/COMMON.ANIMATED.BACKGROUND.PNG")
  label(0, 3, "background.rs · 页面底层", size: 10pt, fill: accent)
  panel(155, 81, 80, 80, fill: ink, stroke: 0.6pt+accent)
  raster(160, 76, 70, 70, "browse/AUDIO.NOCOVER.5.LARGE.PNG", fit: "cover")
  label(155, -4, "artwork.rs · 图库占位缩略图", size: 10pt, fill: accent)
})

#v(8mm)
#grid(columns: (1fr, 1fr), gutter: 9mm,
  note("background.rs", [底图和流光都是绝对定位、铺满窗口。底图使用 Fill 拉伸；流光在 28 s 周期内偏移 ±16 / ±9 设计像素，透明度在 0.06–0.18 间变化。关闭动效后使用静态透明度 0.12。]),
  note("artwork.rs", [用于 136 × 136 图库缩略图。音乐库使用 400 × 400 的原版紫色 AUDIO.NOCOVER.5.LARGE 默认封面并 Cover 裁切；照片与视频使用原版图标并 Contain。空库不展示首页封面拼贴作为真实媒体。]),
)

#v(5mm)
#canvas(length: 1mm, {
  panel(0, 22, 80, 20, fill: ink)
  raster(0, 22, 80, 20, "backgrounds/COMMON.BACKGROUND.LAYER3A.PNG")
  draw.line((86, 12), (109, 12), mark: (end: ">"), stroke: 0.8pt+accent)
  label(116, 20, [*独立流光层*\ Common.Background.Layer3A.PNG\ 叠在底图之上，移动并改变透明度。], width: 130mm)
})

#pagebreak()
#chapter("COMPONENTS / 02", "快捷入口与玻璃框：内容和选择器分离", "shortcut.rs 输出图片 / 图标；focus_frame.rs 只绘制固定玻璃框与亮斑，叠加位置由 home/tiles.rs 决定。")

#canvas(length: 1mm, {
  panel(0, 61, 94, 53, fill: ink)
  raster(16, 53, 62, 35, "shortcuts/STARTMENU.QUICKLINK.MUSIC.PLAYFAVS.NOFOCUS.PNG", fit: "contain")
  label(0, 2, "未选中 · NOFOCUS", size: 10pt, fill: accent)
  draw.line((105, 36), (132, 36), mark: (end: ">"), stroke: 1pt+accent)
  label(118.5, 28, "交叉淡入", size: 8pt, anchor: "north")
  panel(143, 61, 94, 53, fill: ink)
  focus-art(143, 61, 94, "MUSIC.PLAYFAVS")
  label(143, 2, "选中 · FOCUS + 固定玻璃框", size: 10pt, fill: accent)
})

#v(6mm)
#text(weight: "bold", fill: accent)[focus_frame.rs · 分层拆开看]
#v(3mm)
#canvas(length: 1mm, {
  for i in range(3) { panel(i*88, 43, 80, 45, fill: ink) }
  focus-art(0, 43, 80, "MUSIC.PLAYFAVS", frame: false)
  raster(88, 43, 80, 45, "glass/STARTMENU.QUICKLINK.BACKGROUND.PNG")
  raster(187, 41, 48, 3, "glass/SELECTOR.FOCUSLOOP.HOTSPOT.HORIZONTAL.PNG")
  raster(252, 34, 3, 26, "glass/SELECTOR.FOCUSLOOP.HOTSPOT.VERTICAL.PNG")
  label(0, -3, "① 清晰前景 + 模糊底纹", size: 8pt)
  label(88, -3, "② 透明玻璃框 / 高光边缘", size: 8pt)
  label(176, -3, "③ 原版横向、纵向亮斑", size: 8pt)
})

#v(4mm)
#note("几何与合成", [显示尺寸为 238 × (238 × 145 / 258)，保持原版 258:145 比例。外框不属于移动卡片；亮斑沿上边缘缓慢移动。模糊已经在原 FOCUS 位图中，不会每帧计算，也不会实时采样桌面。新增入口复用原版独立图标与同一个玻璃框。])

#pagebreak()
#chapter("COMPONENTS / 03", "按钮：三种状态与真实点击范围", "icon.rs 的 resource_button 使用原版 NOFOCUS / FOCUS / DISABLED 位图；mod.rs 统一导出组件。")

#canvas(length: 1mm, {
  for (i, state) in ("NOFOCUS", "FOCUS", "DISABLED").enumerate() {
    let x = i*86
    panel(x, 55, 78, 48, fill: ink)
    control(x+7, 48, 29, 29, "GOBACK", state: state)
    control(x+43, 48, 29, 29, "PLAY", state: state)
    label(x, 2, ("普通 · NOFOCUS", "悬停 · FOCUS", "禁用 · DISABLED").at(i), size: 10pt, fill: accent)
  }
})

#v(4mm)
#canvas(length: 1mm, {
  panel(0, 32, 35, 35, fill: ink, stroke: (paint: accent, thickness: 0.8pt, dash: "dashed"))
  control(0, 32, 35, 35, "GOBACK", state: "DISABLED")
  draw.line((48, 20), (68, 20), mark: (end: ">"), stroke: 0.8pt+accent)
  label(75, 30, [*42 × 42 是整张图和按钮的边界*\ 返回按钮的圆形主体约 28 × 28；四周透明边距也占空间。\ 文档中的虚线表示布局 / 命中范围，不是额外绘制的 UI 边框。], width: 174mm)
})

#v(4mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("resource_button", [普通位图铺满按钮，保持图片比例。可用按钮叠加悬停位图；不可用按钮直接使用禁用位图。它只负责外观与点击范围，具体动作由 chrome.rs / transport.rs 绑定。]),
  note("icon 与调用方", [icon 仅展示位图；resource_button 的尺寸由调用方提供，顶部返回为 42 × 42。图库播放 / 搜索命令另用 Browse.CMD 资源，与媒体条按钮分开。]),
)

#v(4mm)
#table(columns: (34mm, 1fr, 1fr),
  [*文件*], [*视觉职责*], [*组合位置*],
  [background.rs], [原版底图与独立流光], [home/mod.rs 的最底层],
  [artwork.rs], [图库缩略图 / 独立图标], [home/destination/library.rs],
  [shortcut.rs], [FOCUS / NOFOCUS 的入口内容], [home/tiles.rs 的移动层],
  [focus_frame.rs], [固定玻璃框与边缘亮斑], [home/tiles.rs 的最上层],
  [icon.rs / mod.rs], [原版按钮状态 / 统一组件导出], [chrome · transport · destination],
)


#pagebreak()
#chapter("COMPONENTS / 04", "menu_skin.rs：原图九宫格与菜单字体", "外框、阴影、上下明暗分区、焦点和光泽从原版 PNG 读取；切片缓存复用，边角保持原始大小。")
#canvas(length: 1mm, {
  raster(0, 76, 62, 70, "context-menu/OVERLAYS.OUTLINE.PNG")
  raster(72, 76, 62, 70, "context-menu/OVERLAYS.BACKGROUND.TOP.LIGHT.PNG")
  raster(144, 76, 62, 70, "context-menu/OVERLAYS.BACKGROUND.BOTTOM.DARK.PNG")
  raster(216, 64, 52, 11, "context-menu/COMMON.BUTTON.LEFT.FOCUS.PNG")
  label(0, 0, "外框", size: 9pt)
  label(72, 0, "项目操作 · 浅色", size: 9pt)
  label(144, 0, "通用操作 · 深色", size: 9pt)
  label(216, 44, "焦点高亮", size: 9pt)
})
#v(6mm)
#table(columns: (47mm, 1fr, 1fr),
  table.header([*图层*], [*切分 / 参数*], [*职责*]),
  [Overlays.Outline], [原图 287×86；x=4/283，y=4/82。], [外框 padding 5；最大宽 287。],
  [上下背景], [源图四边 NineGrid 10；操作组 padding 9。], [项目操作与共用操作使用不同深浅图片。],
  [焦点与光泽], [x=20/231，y=12/41；光泽 3s 循环 0 → 0.63 → 0。], [只在选中行叠加；背景 / 字体不通过通用圆角卡片替代。],
  [字体], [segmcsb.ttf：Segoe Media Center Semibold；原版 20pt。], [按 IL 的 points × 96 / 72 转换像素；歌词仍使用系统字体。],
)
#v(4mm)
#note("图示边界", [本页展示资源与层次。CeTZ 图片被直接缩放用于说明，运行时的九宫格由 menu_skin.rs 按原图分割渲染。原始资源字节未改色或重新绘制。])

#pagebreak()
#chapter("COMPONENTS / 05", "默认封面与图库焦点", "artwork.rs 为图库、详细信息和右键上下文共享缺封面资源；library.rs 管理鼠标 / 键盘焦点及当前值接续。")
#canvas(length: 1mm, {
  panel(0, 81, 112, 78, fill: ink)
  raster(20, 67, 44, 44, "browse/AUDIO.NOCOVER.5.LARGE.PNG", fit: "cover")
  label(0, -3, "普通：1.0 / alpha 0.85", size: 10pt, fill: accent)
  panel(139, 81, 112, 78, fill: ink)
  raster(153.28, 72.72, 55.44, 55.44, "browse/AUDIO.NOCOVER.5.LARGE.PNG", fit: "cover")
  panel(153.28, 72.72, 55.44, 55.44, fill: none, stroke: 0.5pt+ice)
  label(139, -3, "焦点：1.26 / alpha 1.0", size: 10pt, fill: accent)
})
#v(7mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("默认图修正", [AUDIO.ALBUMBOX.NOCOVER 是 606×106 的盒背景条，不能当作方形封面。现在引用 AUDIO.NOCOVER.5.LARGE（400×400）；真实嵌入封面优先。]),
  note("布局与交互", [缩略图围绕中心放大，图库内留 22 单位绘制余量避免边缘裁切，标题预留空间。鼠标移出还原；使用方向键后由键盘选择控制焦点。]),
)
#v(5mm)
#note("高亮与离焦不混用", [图库焦点由位图缩放、透明度、边框及阴影组合；播放页歌词的字形光晕、高斯离焦和逐词羽化是另外一套缓存与渲染链路。])

#pagebreak()
#chapter("COMPONENTS / 06", "复用资源：刷新圆球与播放器菜单", "wmc-assets 的资源清单是位图来源；播放器的 wmc_skin.rs 复用相同九宫格切片规则。")
#canvas(length: 1mm, {
  panel(0, 64, 112, 64, fill: ink)
  raster(14, 60, 92, 51.7, "icons/STARTMENU.QUICKLINK.SYNC.FOCUS.PNG", fit: "contain")
  label(0, -6, "SYNC.FOCUS · 媒体库重新扫描", size: 10pt, fill: accent)
  panel(133, 64, 126, 64, fill: ink)
  raster(140, 57, 112, 47, "context-menu/OVERLAYS.BACKGROUND.TOP.DARK.PNG")
  label(148, 48, "歌词信息", size: 11pt, fill: ice)
  label(148, 34, "搜索歌词", size: 11pt, fill: ice)
  label(148, 20, "匹配封面", size: 11pt, fill: ice)
  label(133, -6, "原版菜单背景 · 九宫格示意", size: 10pt, fill: accent)
})
#v(12mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("刷新按钮", [原图透明留白通过定位和裁切消除。实际命中区为 44×44 DIP；加载时 opacity=0.35、普通为 0.8、悬停为 1。图示不代表运行时九宫格尺寸。]),
  note("播放器来源", [默认图仍由原版资源裁切；用户指定图片、音频内嵌图和旁置图均可成为本地来源。联网补全的原图和来源 JSON 放在系统缓存目录，不属于 wmc-assets，不打包进资源清单。]),
)
#v(6mm)
#note("交互边界", [更多菜单与歌词 / 封面设置使用 WMC 资源皮肤；菜单信息和候选列表来自播放器状态。保存缓存不改动源音乐；“写入歌词和封面”是显式动作并创建 .wmc-backup 备份。])
