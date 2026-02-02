# cryosnap

面向 CLI / Rust library / Node & TypeScript 绑定的代码与终端输出截图工具。

[中文](#中文) | [English](#english)

---

## 中文

### 功能概览

- 代码高亮（syntect 主题）
- ANSI 终端输出解析（含 `--execute` PTY）
- SVG / PNG / WebP 输出
- PNG 无损优化（oxipng）
- PNG 有损量化（libimagequant，可选，支持预设）
- 可选多输出：`out.{svg,png,webp}`
- 交互式配置（`--interactive`）
- 配置文件：`default / base / full / user / 自定义路径`
- 字体嵌入（TTF/WOFF/WOFF2）
- Nerd Font（内置 Hack Nerd Font）
- 标题栏（自动文件路径 / tmux 信息）
- 栅格化后端：纯 Rust / rsvg-convert（自动检测）
- 细节配置：padding/margin/border/shadow/line-height/lines/wrap/line-numbers

### 安装

crates.io 安装（推荐）：

```bash
# CLI
cargo install cryosnap

# Rust 库
cargo add cryosnap-core
```

源码安装：

```bash
cargo install --path crates/cryosnap-cli
# 或
cargo build --release
```

### CLI 使用

```bash
# 代码文件 -> PNG
cryosnap main.rs -o out.png

# stdin -> SVG
cat main.rs | cryosnap --language rust -o out.svg

# ANSI 命令输出
cryosnap --execute "eza -lah" -o out.png

# 一次生成多种格式
cryosnap main.rs --output out.{svg,png,webp}

# tmux capture-pane -> PNG (zsh 需转义 %)
cryosnap --tmux --tmux-args "-t %3 -S -200 -J" --config full -o out.png
```

默认行为：stdout 为 TTY 且未指定 `--output/--format` 时，会写入 `cryosnap.png`。

查看完整参数：

```bash
cryosnap --help
```

### 配置

内置配置：

```bash
cryosnap --config base
cryosnap --config full
cryosnap --config user
```

配置文件路径优先级：

1. `CRYOSNAP_CONFIG_PATH`
2. `CRYOSNAP_CONFIG_DIR`（会读取 `user.json`）
3. 系统配置目录（`ProjectDirs`）

### PNG 压缩

默认使用无损优化（不会影响画质）：

```bash
# 关闭无损优化
cryosnap main.rs -o out.png --png-opt=false

# 调整优化级别（0-6）
cryosnap main.rs -o out.png --png-opt-level 4

# 控制元数据剥离
cryosnap main.rs -o out.png --png-strip safe
```

有损量化（体积更小，轻微画质损失）：

```bash
# 开启量化
cryosnap main.rs -o out.png --png-quant

# 使用预设（fast/balanced/best）
cryosnap main.rs -o out.png --png-quant-preset balanced

# 自定义质量/速度/抖动
cryosnap main.rs -o out.png --png-quant-quality 85 --png-quant-speed 4 --png-quant-dither 1
```

### 栅格化后端

默认使用纯 Rust 渲染（resvg/tiny-skia）。如系统已安装 `rsvg-convert`，可选用：

```bash
# 自动检测（找到 rsvg-convert 则使用）
cryosnap main.rs -o out.png --raster.backend auto

# 强制使用 rsvg-convert（未安装会报错）
cryosnap main.rs -o out.png --raster.backend rsvg

# 强制使用纯 Rust
cryosnap main.rs -o out.png --raster.backend resvg
```

### 字体与 Nerd Font

内置 `Hack Nerd Font`，可直接通过 `font.family` 使用：

```bash
cryosnap main.rs -o out.png --font.family "Hack Nerd Font"
```

### 标题栏

默认启用，文件输入时显示**绝对路径**；tmux 模式显示：
`#{session_name}:#{window_index}.#{pane_index} #{pane_title}`。

```bash
# 关闭标题
cryosnap main.rs -o out.png --title=false

# 覆盖标题文本
cryosnap main.rs -o out.png --title.text "hello"

# tmux 标题格式（自定义）
cryosnap --tmux --tmux-args "-t %7 -S -200 -J" \
  --title.tmux-format "#{session_name}:#{window_name} · #{pane_current_path}" \
  -o out.png
```

### 性能建议

```bash
# 降低栅格缩放
cryosnap main.rs -o out.png --raster.scale 2

# 限制最大像素数（0 表示不限制）
cryosnap main.rs -o out.png --raster.max-pixels 8000000

# 如需极致速度可关闭 PNG 优化
cryosnap main.rs -o out.png --png-opt=false
```

### Rust 库用法

```rust
use cryosnap_core::{Config, InputSource, OutputFormat, RenderRequest};

let request = RenderRequest {
    input: InputSource::Text("fn main() {}".to_string()),
    config: Config::default(),
    format: OutputFormat::Svg,
};

let result = cryosnap_core::render(&request).unwrap();
std::fs::write("out.svg", result.bytes).unwrap();
```

### Node / TypeScript 绑定

```bash
cd crates/cryosnap-node
npm install
npm run build
```

发布版（npm）：

```bash
npm install cryosnap
```

```ts
import { render, renderToFile } from "cryosnap";

const bytes = render({
  input: "console.log('hi')",
  inputKind: "text",
  config: {
    theme: "charm",
    window: true,
    showLineNumbers: true,
    padding: [20, 40, 20, 20],
    lineHeight: 1.2
  },
  format: "png"
});

require("fs").writeFileSync("out.png", bytes);

renderToFile(
  {
    input: "console.log('hi')",
    inputKind: "text",
    config: { theme: "charm" }
  },
  "out.webp"
);
```

### 开发与测试

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --ignore-filename-regex "cryosnap-node" --fail-under-lines 80
```

### 发布流程（cargo-release + cargo-dist）

```bash
# 预演
cargo release patch --workspace --dry-run

# 执行（会创建 tag vX.Y.Z）
cargo release patch --workspace --execute

# tag 会触发 GitHub Actions 的 cargo-dist 发布产物
```

crates.io 发布顺序：

```bash
cargo publish -p cryosnap-core
cargo publish -p cryosnap
```

npm 发布：

```bash
cd crates/cryosnap-node
npm publish
```

### 许可证

MIT，详见 `LICENSE`。

### 第三方许可证

- JetBrains Mono 字体：SIL Open Font License 1.1（见 `assets/JetBrainsMono-OFL-1.1.txt`）
- Hack Nerd Font：见 `assets/HackNerdFont-LICENSE.md`

---

## English

### Overview

- Syntax highlighting (syntect themes)
- ANSI terminal capture (with `--execute` PTY)
- SVG / PNG / WebP output
- PNG lossless optimization (oxipng)
- PNG lossy quantization (libimagequant, optional, with presets)
- Multi-output pattern: `out.{svg,png,webp}`
- Interactive config (`--interactive`)
- Config files: `default / base / full / user / custom`
- Font embedding (TTF/WOFF/WOFF2)
- Nerd Font (built-in Hack Nerd Font)
- Title bar (auto file path / tmux metadata)
- Raster backends: pure Rust / rsvg-convert (auto-detect)
- Detailed styling: padding/margin/border/shadow/line-height/lines/wrap/line-numbers

### Install

From crates.io (recommended):

```bash
cargo install cryosnap
cargo add cryosnap-core
```

From source:

```bash
cargo install --path crates/cryosnap-cli
# or
cargo build --release
```

### CLI Usage

```bash
# File -> PNG
cryosnap main.rs -o out.png

# stdin -> SVG
cat main.rs | cryosnap --language rust -o out.svg

# ANSI command output
cryosnap --execute "eza -lah" -o out.png

# Multi-format output
cryosnap main.rs --output out.{svg,png,webp}

# tmux capture-pane -> PNG (escape % in zsh)
cryosnap --tmux --tmux-args "-t %3 -S -200 -J" --config full -o out.png
```

Default behavior: when stdout is a TTY and `--output/--format` is not provided, it writes `cryosnap.png`.

Show all options:

```bash
cryosnap --help
```

### Configuration

Built-in configs:

```bash
cryosnap --config base
cryosnap --config full
cryosnap --config user
```

Config path priority:

1. `CRYOSNAP_CONFIG_PATH`
2. `CRYOSNAP_CONFIG_DIR` (reads `user.json`)
3. System config directory (`ProjectDirs`)

### PNG Optimization

Lossless optimization (no quality loss):

```bash
# Disable lossless optimization
cryosnap main.rs -o out.png --png-opt=false

# Optimize level (0-6)
cryosnap main.rs -o out.png --png-opt-level 4

# Metadata stripping
cryosnap main.rs -o out.png --png-strip safe
```

Lossy quantization (smaller size with slight quality loss):

```bash
# Enable quantization
cryosnap main.rs -o out.png --png-quant

# Preset (fast/balanced/best)
cryosnap main.rs -o out.png --png-quant-preset balanced

# Custom quality/speed/dither
cryosnap main.rs -o out.png --png-quant-quality 85 --png-quant-speed 4 --png-quant-dither 1
```

### Raster Backend

Default uses pure Rust (resvg/tiny-skia). If `rsvg-convert` is available, you can opt in:

```bash
cryosnap main.rs -o out.png --raster.backend auto
cryosnap main.rs -o out.png --raster.backend rsvg
cryosnap main.rs -o out.png --raster.backend resvg
```

### Fonts & Nerd Font

Built-in `Hack Nerd Font` can be used via `font.family`:

```bash
cryosnap main.rs -o out.png --font.family "Hack Nerd Font"
```

### Title Bar

Enabled by default. For file input it shows the **absolute path**; for tmux it shows:
`#{session_name}:#{window_index}.#{pane_index} #{pane_title}`.

```bash
# Disable title
cryosnap main.rs -o out.png --title=false

# Override title text
cryosnap main.rs -o out.png --title.text "hello"

# Custom tmux title format
cryosnap --tmux --tmux-args "-t %7 -S -200 -J" \
  --title.tmux-format "#{session_name}:#{window_name} · #{pane_current_path}" \
  -o out.png
```

### Performance Tips

```bash
# Lower raster scale
cryosnap main.rs -o out.png --raster.scale 2

# Limit max pixels (0 = no limit)
cryosnap main.rs -o out.png --raster.max-pixels 8000000

# Disable PNG optimization for max speed
cryosnap main.rs -o out.png --png-opt=false
```

### Rust Library Usage

```rust
use cryosnap_core::{Config, InputSource, OutputFormat, RenderRequest};

let request = RenderRequest {
    input: InputSource::Text("fn main() {}".to_string()),
    config: Config::default(),
    format: OutputFormat::Svg,
};

let result = cryosnap_core::render(&request).unwrap();
std::fs::write("out.svg", result.bytes).unwrap();
```

### Node / TypeScript Bindings

```bash
cd crates/cryosnap-node
npm install
npm run build
```

Published version (npm):

```bash
npm install cryosnap
```

```ts
import { render, renderToFile } from "cryosnap";

const bytes = render({
  input: "console.log('hi')",
  inputKind: "text",
  config: {
    theme: "charm",
    window: true,
    showLineNumbers: true,
    padding: [20, 40, 20, 20],
    lineHeight: 1.2
  },
  format: "png"
});

require("fs").writeFileSync("out.png", bytes);

renderToFile(
  {
    input: "console.log('hi')",
    inputKind: "text",
    config: { theme: "charm" }
  },
  "out.webp"
);
```

### Development & Testing

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo llvm-cov --workspace --ignore-filename-regex "cryosnap-node" --fail-under-lines 80
```

### Release Flow

```bash
cargo release patch --workspace --dry-run
cargo release patch --workspace --execute
```

Pushing the tag will trigger GitHub Actions to publish cargo-dist artifacts.

Publish to crates.io:

```bash
cargo publish -p cryosnap-core
cargo publish -p cryosnap
```

Publish to npm:

```bash
cd crates/cryosnap-node
npm publish
```

### License

MIT, see `LICENSE`.

### Third-party Licenses

- JetBrains Mono font: SIL Open Font License 1.1 (see `assets/JetBrainsMono-OFL-1.1.txt`)
- Hack Nerd Font: see `assets/HackNerdFont-LICENSE.md`
