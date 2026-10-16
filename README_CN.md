<div align="center">
  <img src="./public/favicon.svg" alt="Screenbox 图标" width="88" />
  <h1>Screenbox</h1>
  <p>录制屏幕，让每个细节更清晰。</p>
  <p>
  <a href="./README.md">English</a> ·
  <a href="https://github.com/morsoli/screenbox/releases/latest">下载</a> ·
  <a href="#实现">实现</a> ·
  <a href="#本地运行">本地运行</a>
  </p>
  <p>
  <img src="https://img.shields.io/badge/macOS-15%2B-111111?logo=apple&logoColor=white" alt="macOS 15 或更高版本" />
  <img src="https://img.shields.io/badge/Tauri-2-FFC131?logo=tauri&logoColor=white" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/React-19-61DAFB?logo=react&logoColor=111111" alt="React 19" />
  <img src="https://img.shields.io/badge/TypeScript-6-3178C6?logo=typescript&logoColor=white" alt="TypeScript 6" />
  <a href="https://github.com/morsoli/screenbox/releases/latest"><img src="https://img.shields.io/github/downloads/morsoli/screenbox/total?label=下载量" alt="GitHub 下载量" /></a>
  <img src="https://img.shields.io/badge/发行包-macOS_.pkg-4C7D5A?logo=apple&logoColor=white" alt="macOS 安装包" />
  <img src="https://img.shields.io/badge/发布工作流-GitHub_Actions-2088FF?logo=githubactions&logoColor=white" alt="GitHub Actions 发布工作流" />
  </p>
</div>

---

Screenbox 是一款个人使用的 macOS 屏幕录制应用。它支持录制屏幕、窗口或单显示器区域，之后可裁剪、缩放、跟随光标、突出显示光标、添加文字标注，并导出本地 MP4 或 WebM。

## 下载与发布

从 [GitHub Releases](https://github.com/morsoli/screenbox/releases/latest) 下载最新 macOS `.pkg` 安装包。每个版本提供 Apple 芯片（`arm64`）和 Intel（`x86_64`）构建。

发布新版本时，推送版本标签（例如 `git tag v0.1.1 && git push origin v0.1.1`）。GitHub Actions 会构建并验证两个架构的安装包，然后附加到该 Release。若在 `main` 分支手动运行工作流，只会生成可下载的 Actions 构建产物；从 `v*` 标签手动运行才会发布 Release。

## 本地运行

需要 **macOS 15 或更高版本**、Xcode Command Line Tools、Rust、Node.js 22+、CMake 和 `PATH` 中的 ffmpeg。在项目目录运行：

```sh
./dev.sh
```

脚本会检查构建依赖，编译 Swift ScreenCaptureKit 辅助程序和固定版本的 whisper.cpp 命令行程序，安装 npm 依赖，然后启动 Tauri 应用。首次构建可能需要几分钟。

如需调整本地路径，可将 `.env.example` 复制为 `.env` 后修改。`.env` 已加入 Git 忽略列表；不要把凭据写入受版本控制的文件。Screenbox 本身不需要凭据。

## 界面设置

在顶部打开**设置**，可选择**跟随系统**、**简体中文**或 **English**，也可选择**跟随系统**、**浅色**或**深色**外观。语言和外观默认跟随 macOS，设置保存在本机 Screenbox 数据库中。切换语言会立即更新主窗口、录制计时器、菜单栏、状态与错误提示，以及之后生成的转录和摘要 Markdown 标题；语音识别出的原文保持录制时的语言。

## 开始录制

1. 选择**全屏**、**窗口**或**区域**。区域录制可在显示器示意图中拖动选择范围。
2. 如果需要录入人声，打开**麦克风音频**后开始录制。首次使用时，macOS 会请求屏幕录制和麦克风权限。
3. 可通过悬浮计时器、应用窗口或菜单栏停止录制。选择录制项目，调整裁剪和画面效果，然后导出。

默认导出位置是 `~/Movies/Screenbox/Exports/`，也可以为单次导出选择其他文件夹。每个视频旁边会生成海报图片、转录 Markdown 和摘要 Markdown。首次转录时会下载多语言 whisper.cpp 模型到本机；模型缓存后，转录可离线运行。模型下载或转录失败时，视频仍会导出，并显示可重试的错误信息。

导出完成后可复制视频路径，或在 Finder 中显示文件。

## 实现

1. **录制：** 用 Swift 原生辅助进程调用 ScreenCaptureKit，提供屏幕、窗口、单显示器区域录制及可选麦克风；保存原始视频和带时间戳的光标轨迹。React 显示悬浮计时器，菜单栏同步显示时间并提供停止操作。处理权限拒绝、设备消失和采集中断，保留可恢复的录制文件。[Apple 的采集示例](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos?language=objc)、[区域采集配置](https://developer.apple.com/documentation/screencapturekit/scstreamconfiguration/sourcerect?language=objc)支持这一路径。
2. **非破坏性编辑：** SQLite 保存项目索引与编辑文档；原片始终保留。编辑文档包含裁剪时间段、画面裁切、缩放、光标高亮、文字标注及导出预设。自动取景依据光标轨迹生成有边界、平滑过渡的关键帧；轨迹不可用时保持居中并在界面说明原因。
3. **本地导出：** Rust 后端校验编辑参数，生成 ffmpeg 滤镜与安全的临时文件，再原子写入成品。提供 H.264 MP4（`libx264`、AAC）及 VP9 WebM（`libvpx-vp9`、Opus）预设；同名导出旁生成海报帧、转录 Markdown 和摘要 Markdown。文字标注使用转义后的字幕文件，避免把用户输入拼入命令行。[ffmpeg 滤镜文档](https://www.ffmpeg.org/ffmpeg-filters.html)涵盖缩放与字幕渲染。
4. **转录与文件操作：** 首次需要转录时下载多语言 `whisper.cpp` 本地模型，显示进度并缓存；摘要从转录内容提取带时间戳的要点。无麦克风录制仍生成注明“无语音”的转录和录制信息摘要。默认导出到 `~/Movies/Screenbox/Exports/`，支持自选目录、复制文件路径、在 Finder 中显示。[whisper.cpp](https://github.com/ggml-org/whisper.cpp)支持离线推理。
5. **架构与本地数据：**`native/Capture.swift` 使用 ScreenCaptureKit 和 AVAssetWriter 录制原片及光标轨迹，并由 Tauri Rust 后端控制；`src-tauri/src/` 负责 SQLite 项目目录、偏好设置、输入校验、录制生命周期、ffmpeg 渲染、转录及 Finder／剪贴板操作；`src/` 是 React 与 TypeScript 界面。应用数据保存在 `~/Library/Application Support/com.morso.screenbox/`，包括项目、编辑文档和偏好设置数据库 `screenbox.sqlite3`、`recordings/<id>/original.mp4`、光标轨迹、模型缓存和临时渲染文件。退出 Screenbox 后，复制此目录及使用过的导出目录即可备份；恢复时在启动应用前复制回原位置。录制、转录和导出内容默认留在本机，不上传视频、文本或使用数据。

## 测试

```sh
npm run test:unit
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:e2e
npm run build:web
```