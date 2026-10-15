// typst compile --root . crates/wmc-player/preview.typ target/ui-docs/player.pdf
#import "docs/player-diagrams.typ": *
#show: doc-style

#chapter("WMC PLAYER / 01", "播放页：宽屏双列与紧凑布局", "2026-10-14 核对 view/layout.rs、frame.rs、metadata.rs；CeTZ 结构图，非运行截图。背景色、离焦和字体仅作示意。")
#canvas(length: 1mm, {
  player-view(s: 0.14)
  label(190, 90, [*宽屏参考*\ 1280×720 DIP\ 封面 378×378\ 歌词视口锚点 42%], size: 10pt, width: 69mm)
  label(190, 45, [*紧凑模式*\ 顶端小封面与元信息\ 下方完整歌词区域\ 媒体条留出底部空间], size: 10pt, width: 69mm)
})
#v(5mm)
#table(columns: (43mm, 1fr, 1fr),
  table.header([*布局项*], [*当前参数*], [*解释*]),
  [宽屏条件], [宽 / 高 ≥ 1.3 且宽 ≥ 1024 DIP。], [宽屏缩放 s = min(w / 1376, h / 1030)；几何与排字缩放分开。],
  [封面], [边长 = h × 0.42 × 1.25；中心在 (w / 4, h × 0.4)。], [1280×720 时为 378×378；元信息位于封面下方 h × 0.04。],
  [歌词], [字号 48、行高 60、行间隔 40，均使用排字缩放。], [歌词列左缘 w / 2 + 18；底部留 52 DIP 给媒体条；锚点在歌词视口高度的 42%。],
  [元信息与进度], [歌名 20 粗体、艺术家 18；右侧“⋯”打开更多菜单。], [灰色轨道、白色进度，拖动时显示圆形游标。时间以真实播放位置计算。],
  [紧凑窗口], [封面缩为顶端小图，元信息位于右侧；歌词在下方。], [宽屏条件不成立时使用 626×1382 的缩放参考，s 最小 0.75；窗口最小 520×540。],
)
#v(3mm)
#note("控件自动隐藏", [鼠标移出后 2 秒、窗口内静止 5 秒，控制区以 250ms 淡出；鼠标活动或正在交互时恢复。光标隐藏由保留的平台句柄处理，宿主可同步窗口顶部控件透明度。])

#pagebreak()
#chapter("WMC PLAYER / 02", "歌词时间轴：逐字、多人声部与伴奏", "lyrics/ 负责格式与时间，view/lyrics/ 负责排字、纹理和绘制；音频时钟来自 audio/clock.rs。")
#canvas(length: 1mm, {
  flow-box(0, 63, "音频时钟", "audio/source.rs + clock.rs\n真实位置校准连续显示时间", width: 82)
  flow-box(94, 63, "统一歌词模型", "lyrics/lrc.rs + ttml.rs\nLine / Word / agent / group", width: 82)
  flow-box(188, 63, "显示与运动", "timeline.rs + presentation.rs\n独立声部、弹簧和逐字进度", width: 82)
  for x in (82, 176) { draw.line((x,49),(x+12,49),mark:(end:">"),stroke:0.8pt+accent) }
  label(0, 18, [*呼吸点*\ 三个圆同步缩放，周期 3.2 秒；从左到右由灰变蓝。], width: 125mm)
  for i in range(3) { draw.circle((153+i*13, 12), radius: 3.5, fill: if i == 0 {accent} else {muted}) }
})
#v(5mm)
#table(columns: (43mm, 1fr, 1fr),
  table.header([*输入 / 机制*], [*行为*], [*边界*]),
  [纯文本与 LRC], [纯文本不伪造时间；普通 LRC 按行点亮；增强 LRC 使用词边界。], [没有演唱结束信息的逐行 LRC，不能准确识别行内伴奏。],
  [TTML], [支持 media 时间、词时间、演唱者和背景声部；兼容 Apple 绝对时间及 AMLL 混合命名空间。], [已知翻译 / 音译角色不混入主唱；原文保存在缓存。seq、帧 / tick 时间和 br 等限制仍按解析器诊断。],
  [同时高亮], [最多两行主唱，伴唱独立；以实际演唱区间选择，保留稳定分组和右对齐声部。], [锚点优先最早仍在演唱的主唱；伴唱不抢占主唱名额。],
  [跨字连续], [progression.rs 用连续羽化跨过词边界；空档保持游标，定位重新采样。], [不把离散词开始时刻作为重启动画的触发器。],
  [间奏提示], [合并全部声部的演唱区间；开头和中间至少 3 秒的空档显示圆点。], [暂停冻结；下一句前 650ms 淡出并释放布局空间；最后一句之后不显示。],
)
#v(3mm)
#note("参考参数与实现", [50ms 错峰、100ms 滚动提前、弹簧参数等来源见 APPLE_MUSIC_REFERENCE.md。当前锚点为歌词视口 42%，不是此前文档的整窗中线；字体与场景几何以 Rust 当前值为准。])

#pagebreak()
#chapter("WMC PLAYER / 03", "加载与持久化：本地优先，缺失才联网", "track/prepare.rs 完成本地准备；view/media_cache.rs 在后台计算文件 MD5 并调度歌词和封面的独立补全。")
#canvas(length: 1mm, { preparation-flow() })
#v(5mm)
#table(columns: (43mm, 1fr, 1fr),
  table.header([*步骤*], [*当前行为*], [*数据 / 隔离*]),
  [本地来源], [显式参数优先，其次音频内嵌歌词和封面、同名 TTML / LRC 及本地图片。], [显式来源不被自动网络结果覆盖；示例合成曲目不自动联网。],
  [缓存查找], [先匹配音频文件 MD5，再匹配歌名和艺术家。], [同名候选再参考专辑、时长和更新时间；命中有效缓存时不联网。],
  [自动补全], [歌词和封面分别判断缺失，按排序取第一条可用结果；失败继续尝试，最多五项。], [请求、哈希、解析及缓存 I/O 在后台；过期代次不会覆盖新选择。],
  [总索引], [系统缓存目录下 media-cache/index.json。], [记录音频 MD5、路径、标题、艺术家、专辑、时长、本地资源状态和来源文件指针。],
  [资产文件], [`lyrics/*.json` 保存歌词原文和翻译；`covers/*.json` 保存图片 Base64 和来源。], [总索引保留实际候选歌手 / 专辑、来源 ID / URL 和获取时间；文件锁与原子替换防止并发覆盖。],
)
#v(4mm)
#note("来源与格式", [网易云、QQ 提供无需账号的搜索与 LRC；AMLL 提供社区 TTML，主站下载失败可走 GitHub 镜像。封面搜索还使用公开 iTunes 目录。当前未接网易云加密 YRC、QQ 加密 QRC 和 Apple 认证歌词；接口返回与歌曲版本仍需核对。])
#v(3mm)
#note("路径与修改源文件", [Windows 缓存根为 LOCALAPPDATA / Windows-Media-Center；macOS 使用 Library/Caches，Linux 使用 XDG_CACHE_HOME 或 ~/.cache。自动缓存不修改音频。歌曲设置中的显式写入会保留 .wmc-backup 备份。])

#pagebreak()
#chapter("WMC PLAYER / 04", "更多菜单、歌词信息与封面设置", "view/more.rs、settings.rs、online_lyrics.rs、online_covers.rs 共用 WMC 九宫格资源皮肤。")
#canvas(length: 1mm, {
  panel(0, 67, 83, 67, fill: brown)
  label(8, 59, "歌曲名称                   ⋯", size: 12pt, fill: ice)
  for (i, name) in ("歌词信息", "搜索歌词", "匹配封面").enumerate() { label(8, 43-i*13, name, size: 10pt, fill: ice) }
  flow-box(99, 67, "歌词设置", "标题 / 艺术家 / 专辑\n来源、时间轴、原始文本\n编辑 / 应用 / 保存缓存", width: 76, height: 47)
  flow-box(188, 67, "封面设置", "候选缩略图 / 来源 / 专辑\n选择后更新封面与动态背景\n同步系统媒体缩略图", width: 81, height: 47)
  draw.line((83,34),(99,34),mark:(end:">"),stroke:0.8pt+accent)
})
#v(5mm)
#table(columns: (43mm, 1fr, 1fr),
  table.header([*操作*], [*效果*], [*约束*]),
  [搜索歌词], [Ctrl+L 或更多菜单，展示候选、歌手、专辑和来源。], [候选排名不保证录音版本相同；可手动改选，并覆盖保存的选择。],
  [匹配封面], [Ctrl+Shift+C 或更多菜单，显示候选缩略图。], [应用后更新封面、背景图层和系统媒体封面；后台连续生成背景帧。],
  [歌词信息], [查看、编辑原始歌词，应用解析结果，支持逐字显示选项。], [编辑状态、可写入源文件和保存状态分开管理；错误保留诊断。],
  [写入歌词和封面], [显式写入当前音乐文件，先保留备份。], [与默认本地缓存独立；demo 没有可写入的源音乐文件。],
  [系统播放控制], [Windows SMTC 发布标题、艺术家、封面、进度；响应系统命令。], [最小化时仍处理命令；系统音量由独立后台 Controller 管理。],
)

#pagebreak()
#chapter("WMC PLAYER / 05", "渲染缓存与模块边界", "当前路径均与重构后的目录对应；对外保留 PlayerConfig / PreparedTrack / PlayerView 等入口。")
#canvas(length: 1mm, { glyph-cache-flow() })
#v(5mm)
#table(columns: (48mm, 1fr, 1fr),
  table.header([*模块*], [*职责*], [*生命周期与边界*]),
  [audio/ 与 track/], [流式音频、连续时钟；标签、歌词、封面及分段准备计时。], [准备不收集整首 PCM；字体和背景计算不阻塞初次音频准备。],
  [lyrics/ 与 motion.rs], [格式解析、声部、时间轴、跨字进度、行运动和间奏提示。], [数据模型独立于 GPUI；暂停与定位按同一音频时间采样。],
  [view/lyrics/], [font.rs / layout.rs / raster.rs / cache.rs / line.rs。], [歌曲字符按需栅格化；尺寸改变替换缓存；后台生成高光与离焦纹理。],
  [background.rs 与资源边界], [封面三层图像变换、PCM 频谱调制及低分辨率模糊。], [最多一个 CPU 生成任务和两个当前 GPU 帧；图集边缘填充保护缩放采样。],
  [network.rs 与缓存], [共享 HTTP 连接、并发与响应大小限制；media_cache.rs 管理 JSON 索引。], [cache_paths.rs 处理平台缓存目录；不将网络图片当成打包资源。],
  [media/ 与 system_volume.rs], [系统媒体会话、命令通道和系统主音量。], [平台专用能力与通用播放器隔离；原生交互测试显式运行。],
)
#v(4mm)
#note("性能数据边界", [LOADING_PERFORMANCE.md 与 MEMORY_CPU_ANALYSIS.md 保留历史单次探针结果；它们不是当前整机或端到端性能承诺。窗口、音频设备、CPU 像素载荷、GPU 图集和驱动内存需分别计数。静止状态仍可能请求动画帧。])

#pagebreak()
#chapter("WMC PLAYER / 06", "验证入口与当前边界", "图解由 Typst 编译；行为由 Rust 单元测试、诊断示例和实际接口联通验证。")
#table(columns: (53mm, 1fr, 1fr),
  table.header([*入口*], [*验证对象*], [*不代表*]),
  [cargo test -p wmc-player --lib], [时间轴、TTML、字形、缓存查找及并发索引等确定性行为。], [原生系统交互测试被显式忽略时，不能声称已验证真实桌面交互。],
  [lyrics-search / cover-search], [真实接口搜索与指定结果下载、解析和图片解码。], [外部网络测试不进入日常离线单元测试；接口可用性会变化。],
  [asset-cache-probe], [读取音乐元信息与 MD5；首次缺失时下载，再次命中缓存不联网。], [缓存能命中不等于录音版本一定正确；实际候选信息记录在 JSON 中。],
  [prepare-bench / glyph-memory-probe], [文件准备分段耗时与歌曲字形 CPU / 内存探针。], [不等于点击到出声耗时或主程序总 GPU / 私有内存。],
  [duet-demo], [项目 TTML 示例的主唱、伴唱及无声预览；可指定真实音频。], [歌词资产对应特定录音，不应套用到所有同名版本。],
)
#v(6mm)
#note("尚未覆盖", [持久播放列表、自动连续播放、完整 RTL 排字、所有 TTML 扩展及认证歌词服务仍需独立实现。不要再将已实现的 TTML / 声部输入列为后续工作。])
#v(3mm)
#note("文档维护", [同时更新 player-diagrams.typ 和本文件；共享首页绘图来自 wmc-ui/docs/ui-diagrams.typ。生成 PDF / PNG 放入 target/ui-docs，不提交本地分析输出、运行缓存或编译产物。])
