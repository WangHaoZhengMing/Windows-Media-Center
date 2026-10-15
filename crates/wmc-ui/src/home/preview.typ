// From the repository root:
// typst compile --root . crates/wmc-ui/src/home/preview.typ target/ui-docs/home.pdf
#import "../../docs/ui-diagrams.typ": *
#show: doc-style

#note("核对日期 · 2026-10-14", [当前实现包含媒体库图标刷新、独立播放器、联网歌词和封面补全。页面为 CeTZ 结构示意，运行时布局以 Rust 为准。])

#chapter("HOME / 01", "首页：固定焦点，流动内容", "mod.rs 组装整页；以下为当前 Rust 实现的示意，不是原程序截图。资源直接引用 wmc-assets。")

#canvas(length: 1mm, {
  home-view(selected: 2)
  callout(1, (25, 108), (218, 110), "chrome.rs", "返回 / 主页\n拖动区 / 窗口按钮")
  callout(2, (32, 78), (218, 87), "rail.rs", "固定顺序的栏目列表\n选中位置保持不变")
  callout(3, (77, 47), (218, 64), "tiles.rs · 选择框", "位置固定的玻璃框\n最后叠在移动内容上方")
  callout(4, (156, 42), (218, 41), "tiles.rs · 快捷入口", "左右切换入口\n每个栏目保留自己的选择")
  callout(5, (140, 9), (218, 18), "transport.rs", "媒体控制常驻右下角\n不改变首页快捷入口")
})

#v(3mm)
#note("mod.rs 的绘制顺序", [背景 → 首页栏目与入口 → destination 页面 → 媒体条 → 整页详细信息 → 顶部窗口控件 → 右键菜单。播放器有独立入场 / 退出路径。窗口按 1280 × 720 设计，等比例适配可用空间。])

#pagebreak()
#chapter("HOME / 02", "顶部窗口区与媒体控制条", "chrome.rs 与 transport.rs 分开绘制；窗口操作、首页返回、媒体操作各自拥有独立语义。")

#canvas(length: 1mm, {
  panel(0, 31, 254, 26, fill: ink)
  control(3, 28, 18, 18, "GOBACK", state: "DISABLED")
  control(25, 28, 18, 18, "MENU")
  panel(51, 28, 137, 18, fill: rgb("12385a"), stroke: (paint: ice, thickness: 0.4pt, dash: "dashed"))
  label(119.5, 19, "空白拖动区 · 双击最大化", size: 10pt, fill: ice, anchor: "center")
  for (i, name) in ("MINIMIZE", "RESTORE", "EXIT").enumerate() {
    control(194+i*19, 27, 17, 16, name)
  }
  label(3, 1, "返回", size: 8pt)
  label(25, 1, "主页", size: 8pt)
  label(194, 1, "最小化 / 还原 / 关闭", size: 8pt)
})

#v(7mm)
#text(weight: "bold", fill: accent)[transport.rs · 无媒体时的实际状态]
#v(3mm)
#canvas(length: 1mm, { transport-view(s: 0.57, y: 29, names: true) })
#v(7mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("窗口区", [返回位图的尺寸为 42 × 42，主页按钮为 40 × 40。只有空白顶部区域是拖动命中区；按钮各自处理点击。最大化后改用还原图标。]),
  note("媒体条", [队列按钮打开播放队列详情。无媒体时播放、停止、跳转、快进和快退显示禁用状态；音量 / 静音保存播放器偏好。音乐文件已经接入 wmc-player 的真实音频后端；打开播放器后使用播放器自己的媒体条。]),
)

#pagebreak()
#chapter("HOME / 03", "栏目切换：同一锚点，不同时序", "rail.rs 与 tiles.rs 消费 wmc-motion 的同一份采样；actions.rs 将按键转换成语义动作。")

#canvas(length: 1mm, {
  for (i, selected) in (1, 2, 3).enumerate() {
    draw.scope({ draw.translate((i*88, 12)); home-view(s: 0.0625, selected: selected) })
    label(i*88, 7, ("视频 →", "音乐 →", "设置").at(i), size: 9pt, fill: accent)
  }
  label(0, -2, "前面的栏目继续留在上方，直到离开视口；三个画面中的玻璃框在同一位置。", size: 9pt)
})

#v(5mm)
#canvas(length: 1mm, {
  let x0 = 55
  let unit = 0.68
  for t in (0, 100, 133, 267) {
    draw.line((x0+t*unit, 3), (x0+t*unit, 35), stroke: 0.4pt+rgb("d3dfe8"))
    label(x0+t*unit, 0, str(t)+" ms", anchor: "north", size: 8pt)
  }
  for (i, item) in (("栏目位移", 0, 267), ("旧栏目退焦", 0, 100), ("新栏目亮起 / 边距", 133, 267)).enumerate() {
    let y = 31 - i * 11
    label(0, y, item.at(0), size: 9pt)
    panel(x0+item.at(1)*unit, y+1, (item.at(2)-item.at(1))*unit, 6,
      fill: (accent, rgb("70a5b7"), rgb("29496a")).at(i))
  }
})

#v(4mm)
#grid(columns: (1fr, 1fr), gutter: 7mm,
  note("交互与状态", [↑ ↓：顺序切换栏目；← →：入口选择；Enter：打开；Esc / Backspace：返回；F11：全屏。每个栏目记住自己的入口，返回恢复选择。]),
  note("还原边界", [栏目字号保持 44；267 ms 位移、100 ms 退焦、133 ms 等待来自原 MCML。曲线公式和入口交叉透明度为 Rust 近似；快速反向从当前采样接续。]),
)

#pagebreak()
#chapter("HOME / 04", "二级页面：图库与命令内容布局", "destination/ 按原版 BrowsePage 与 CommandContentLayout 拆分页面；共用背景、顶部窗口控件和媒体条。")

#canvas(length: 1mm, {
  home-view(s: 0.098, details: (title: "音乐库", eyebrow: "SOUNDTRACK OF YOUR DAY", kind: "music",
    body: "音乐库还是空的。你的下一首好歌，会出现在这里。"))
  draw.scope({
    draw.translate((135, 0))
    home-view(s: 0.098, details: (title: "显示与动效", eyebrow: "YOUR COMFORT", kind: "display",
      body: "缓缓流动的光，让客厅多一份宁静。"))
  })
  label(0, -5, "音乐库 · 本地文件与默认封面", size: 10pt, fill: accent)
  label(135, -5, "显示设置 · 流光开关", size: 10pt, fill: accent)
})

#v(6mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("页面骨架", [媒体库：右上标题、横向分类、缩略图区；分类右侧的原版同步圆球用于重新扫描，扫描中禁用并降低透明度。设置：左侧命令、右侧选项。首页在过渡后完全隐藏；移除大封面和压暗背景。]),
  note("目前的功能范围", [启动时后台扫描本地目录；音乐读取标题、艺术家、专辑和封面并支持真实播放，照片可在主窗口查看，视频交由系统播放器打开。分类页目前主要改变排序，EXIF、评分、标签筛选、专辑聚合和目录实时监视尚未实现。]),
)

#v(5mm)
#table(columns: (30mm, 1fr, 1fr),
  [*文件*], [*负责的界面*], [*数据 / 组件来源*],
  [mod.rs], [首页组装、焦点、菜单 / 详细信息与播放过渡], [wmc-core · wmc-motion · components],
  [chrome.rs], [自定义标题栏与返回 / 主页], [resource_button · 原版控制位图],
  [rail.rs / tiles.rs], [栏目 / 入口与固定玻璃框], [CategoryPose · shortcut · focus_frame],
  [destination/], [图库、队列、设置与关于], [DestinationState · LibraryState · artwork · 原版 Browse 资源],
  [transport.rs / actions.rs], [媒体条 / 快捷键语义], [MediaCommand · HomeState],
)
#pagebreak()
#chapter("HOME / 05", "关于：软件版本与项目身份", "destination/about.rs 仿照 OPTIONS.GLOBAL.ABOUT.XML 的信息层级，展示重写项目的实际信息。")
#canvas(length: 1mm, {
  home-view(s: 0.16, details: (kind: "about"))
  callout(1, (71, 73), (218, 90), "左侧操作", "设置标题 + 完成按钮\n返回已有首页选择")
  callout(2, (91, 80), (218, 65), "标志与软件名", "原版圆球资源\nSegoe UI 轻字重")
  callout(3, (174, 41), (218, 35), "紧凑信息段落", "项目版本 / 界面资源\n运行平台与处理器数量")
})
#v(4mm)
#note("内容来自项目与运行环境", [版本使用 Cargo 配置；平台、架构与逻辑处理器数量在运行时读取。图中“运行时读取”是说明占位。图解不照抄原程序的 Windows 8.1、安装器版本或 Microsoft 版权信息。])


#pagebreak()
#chapter("HOME / 06", "本地媒体库：目录配置与后台扫描", "wmc-library 不依赖 GPUI；home/media_library.rs 把扫描、标签和封面接到主窗口。")
#canvas(length: 1mm, { folder-flow() })
#v(6mm)
#table(columns: (42mm, 1fr, 1fr),
  table.header([*阶段*], [*当前实现*], [*状态与边界*]),
  [目录配置], [设置 → 媒体位置；按照片 / 音乐 / 视频添加目录、移出、恢复默认、保存 / 取消。], [目录先写草稿，保存后生效；明确保存的空列表不会自动回填。],
  [持久化], [系统本地应用数据目录的 WMC/library.json；临时文件替换。], [损坏配置报告错误，读取时不覆盖；移出目录不删除媒体文件。],
  [扫描], [递归、扩展名筛选、路径去重，跳过链接和 reparse points。], [可取消；过期 generation 结果不覆盖新扫描；不可访问目录只影响自身。],
  [展示], [音乐标签与小封面；照片文件缩略图；24 项一页，网格 / 列表切换。], [刷新显示新增文件；分类目前为文件列表排序，不冒充完整筛选或专辑分组。],
)
#v(5mm)
#note("打开媒体", [音乐在后台准备后自动播放；照片进入整页浏览并可左右切换；视频用系统默认播放器打开。返回保留媒体库选择和页码。没有媒体时保留真实空状态，不伪造文件。])

#pagebreak()
#chapter("HOME / 07", "右键菜单：项目操作与通用操作分组", "context_menu.rs 接入真实动作；原图与 IL / MCML 参数保存在 docs/reference。图中背景拉伸仅为结构示意，运行时用九宫格。")
#canvas(length: 1mm, {
  home-view(s: 0.14, details: (kind: "music"))
  context-menu-view(89, 85, 0.13)
  label(194, 90, [*项目操作*\ 播放 / 加入列表\ 详细信息 / 从本次列表移除], width: 72mm)
  label(194, 53, [*通用操作*\ 网格 / 列表\ 管理媒体库 / 设置], width: 72mm)
})
#v(5mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  note("输入和定位", [鼠标右键或 Shift+F10；上下键循环、Enter 执行、Esc / 外部点击关闭。菜单跟随项目，优先右侧、不足时翻左并垂直居中；失去焦点 2 秒后关闭。]),
  note("动作语义", [列表仅保存在当前会话；可添加、清空、点播。随机 / 重复目前只维护选项状态，未实现自动连续播放。移除只影响本次媒体库列表，重新扫描会恢复，磁盘文件不会删除。]),
)
#v(4mm)
#note("展开与收起", [面板 400ms 展开，文字 250ms 后显现、500ms 完成；关闭 250ms 缩至 0.65 并淡出。上下文缩略图 100ms 放大至 1.3，关闭时 150ms 还原。Log / Exp / SCurve 使用实际 IL 公式；页面 Z 轴透视仍未接入 GPUI 投影。])

#pagebreak()
#chapter("HOME / 08", "详细信息：与媒体库一致的整页布局", "details.rs 复用蓝色背景、256 像素封面、玻璃高亮命令和大字号标题；数据在后台读取。示例值不代表实际文件。")
#canvas(length: 1mm, { home-view(s: 0.19, details: (kind: "file-details")) })
#v(5mm)
#table(columns: (36mm, 1fr, 1fr),
  table.header([*媒体*], [*真实信息*], [*操作与异常*]),
  [音乐], [标签标题 / 艺术家 / 专辑；时长、采样率、声道、码率、位深（有值时）。], [读取属性不打开音频设备；缺失字段显示“未提供”。],
  [照片 / 视频], [照片像素尺寸；视频媒体类型。共用文件名、大小、修改时间（UTC+8）及路径。], [尺寸读取失败仍显示文件信息；视频未解析时长或画面缩略图。],
  [整页交互], [播放 / 查看、复制文件路径、返回媒体库；箭头 / Tab / Enter / Esc。], [220ms 入场、180ms 关闭；过期读取不会覆盖重开的页面，文件不可用时禁用播放。],
)

#pagebreak()
#chapter("HOME / 09", "图库悬停与播放页往返", "图库悬停使用原版 100ms 轨迹；player_transition.rs 协调播放器退出和图库各层独立返回。")
#table(columns: (43mm, 1fr, 1fr),
  table.header([*层 / 场景*], [*时序与数值*], [*当前边界*]),
  [鼠标移入 / 移出], [100ms Log 放大到 1.26、透明度 0.85 → 1；100ms Exp 还原。], [中途反向从当前值接续；视口留出余量、标题预留空间；减少动态效果直接到终态。],
  [进入播放器], [图库退出 250ms；封面 700ms 显现；详情 300ms 后显现、1s 完成，位移到 2.6s 结束。], [部分入场曲线仍为近似；后台歌曲准备与视觉入场分别计时。],
  [返回媒体库], [标题 200–400ms，图块 300–500ms，命令 / 分类 333–500ms，页计数至 533ms。], [退出先暂停音频；选中项与其他项独立缩放。返回的 Log / Exp 仍为三次曲线近似。],
)
#v(5mm)
#note("源码与证据分开", [运行实现见 wmc-motion/context_menu.rs、player_entry.rs、player_return.rs 和 home/destination/library.rs。原版声明 / IL 是参数依据；文档示意和编译成功均不等于运行窗口已逐帧达到 1:1。])

#pagebreak()
#chapter("HOME / 10", "媒体库到播放器：本地优先，后台补全", "home/media_library.rs / player_transition.rs 与 wmc-player 共同完成加载和返回；具体歌词图解见播放器 preview.typ。")
#canvas(length: 1mm, {
  folder-flow()
})
#v(7mm)
#table(columns: (43mm, 1fr, 1fr),
  table.header([*入口*], [*当前行为*], [*持久化 / 边界*]),
  [刷新图标], [重新扫描配置中的目录；扫描时禁用重复点击。], [移出的会话文件可由重新扫描恢复；不删除原文件。],
  [点播歌曲], [后台准备音频、标签、同名歌词和图片，进入 PlayerView 后播放。], [过期结果使用代次隔离，不覆盖当前页面。],
  [缺失资源], [已有本地来源和缓存优先，缺失才联网选第一条可用歌词 / 封面。], [缓存按 MD5 优先、歌名和艺术家回退；总索引为 media-cache/index.json。],
  [更多菜单], [歌词信息、搜索歌词、匹配封面；支持查看与编辑来源。], [缓存保存与显式写回音乐文件是不同操作；写回前备份。],
  [系统控制], [播放器发布封面、标题、进度，响应系统播放 / 暂停 / 定位。], [平台支持和验证边界见 SYSTEM_MEDIA.md 与 SYSTEM_VOLUME.md。],
)
