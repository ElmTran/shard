<img src="src-tauri/icons/128x128.png" width="96" alt="Shard 图标" align="right" />

# Shard

[English](README.md) | 中文

一个自用的轻量 Windows 截图工具：截图、标注、贴图、识别文字。界面极简，唤起快，OCR 用系统自带引擎，离线、不增加安装包体积。

## 功能

- **截图**：按 `Alt+A`，拖出选区。选区可以拖动，拖四角或四边调整大小。支持多显示器，按物理像素截取，高分屏下不模糊。
- **标注**：聚光灯（选区内其余部分压暗）、箭头、铅笔、文字、马赛克，可以撤销上一步。
- **复制**：按 `Enter`、双击选区或点 ✓，图片复制到剪贴板（同时写入 DIB 和 PNG 两种格式，微信、Office、浏览器都能粘贴）。
- **贴图**：把选区钉在桌面上，置顶显示，可以拖动。贴图出现后会在后台自动识别文字，不显示识别框，直接拖选文字就能复制。
- **识别文字**：截图时点识别按钮，文字以透明层叠在原位置上，可以框选复制，也可以一键复制全部。
- **保存**：保存为 PNG 文件。

## 快捷键

| 范围 | 按键 | 作用 |
|---|---|---|
| 全局 | `Alt+A` | 截图 |
| 截图窗口 | `Enter` / 双击选区 | 复制并关闭 |
| 截图窗口 | `Ctrl+Z` | 撤销 |
| 截图窗口 | `Ctrl+C` | 复制选中的识别文字；没有选中文字时复制图片 |
| 截图窗口 | 右键 | 有选区时取消选区，否则退出 |
| 截图窗口 | `Esc` | 退出 |
| 贴图 | 拖动 | 移动 |
| 贴图 | 双击 / `Esc` | 关闭 |
| 贴图 | `Ctrl+C` | 复制选中的文字；没有选中文字时复制图片 |
| 贴图 | 右键 | 菜单：复制图片、复制全部文字、保存、关闭 |

`Alt+A` 和微信的截图快捷键相同，PixPin 等截图工具也可能占用它。注册失败时 Shard 会弹窗提示，在对方的设置里改掉即可；在此之前可以点托盘图标截图。

## 技术栈

| 层 | 选择 |
|---|---|
| 壳 | [Tauri v2](https://tauri.app) |
| 前端 | React 19、TypeScript、Tailwind CSS 4、Vite（不用组件库和状态库） |
| 截图 | GDI `BitBlt` 直接写进 WebView2 SharedBuffer 共享内存，前端用 `VideoFrame` 转成位图，不经过 IPC 拷贝 |
| 标注 | Canvas 2D，图形保存为矢量列表，导出时按物理像素合成 |
| OCR | `Windows.Media.Ocr`（系统自带，离线） |
| 系统交互 | `windows` crate：Win32 剪贴板、DWM（隐身显示、关闭窗口动画、圆角） |
| 前后端类型 | ts-rs 从 Rust 类型生成 TypeScript 类型 |
| 打包 | NSIS 安装包，按当前用户安装 |

只支持 Windows 10/11。OCR 语言取决于系统装了哪些识别语言包（中文识别器也能识别英文和数字）。

## 开发

需要 Rust（MSVC 工具链）、Visual Studio Build Tools（"使用 C++ 的桌面开发"）、Node.js 和 pnpm。

```bash
pnpm install
```

```bash
pnpm tauri dev
```

打包，生成 `src-tauri/target/release/bundle/nsis/Shard_<版本>_x64-setup.exe`：

```bash
pnpm tauri build
```

Rust 单元测试（同时更新 `src/lib/bindings/` 下的 TS 类型）：

```bash
cd src-tauri && cargo test
```
