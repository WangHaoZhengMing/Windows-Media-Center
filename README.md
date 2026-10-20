# Windows Media Center

使用 Rust 和 GPUI 构建的桌面媒体中心，目前发布 Windows x64 版本。

## 功能

- 本地音乐播放，支持专辑、艺术家、歌曲分类。
- 逐字歌词、翻译和动态模糊效果，背景随专辑封面配色。
- 字体按 SF Pro Display、微软雅黑及其他系统字体逐级回退，支持韩语等混排。
- Windows 系统媒体控制集成与系统音量调节。
- 照片媒体库：正方形缩略图、时间排序、连续滚动、触控板缩放与按需加载。
- 媒体扫描采用有上限的并行处理，分批显示结果；本次运行内复用未修改文件的元数据缓存。

## 下载和运行

在仓库的 Releases 页面下载 Windows x64 压缩包，解压后运行 `wmc.exe`。
也可以直接下载单独的 `wmc.exe`。在应用设置中添加照片、音乐和视频文件夹。

SF Pro Display 不随程序分发；未安装时会自动选择后续可用字体。
元数据刷新缓存保存在内存中，退出应用后释放。

## 从源码构建

安装支持 Rust 2024 edition 的稳定版 Rust。Windows 构建需要 Visual Studio C++ Build Tools 和 Windows SDK。

```powershell
cargo test --workspace --lib
cargo build --release -p wmc-desktop --bin wmc
```

生成的可执行文件位于 `target/release/wmc.exe`。
