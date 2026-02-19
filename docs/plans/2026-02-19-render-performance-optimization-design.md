# Cryosnap 渲染性能优化设计（CLI / Core / Node）

日期：2026-02-19

## 背景

目前 PNG 渲染路径总体是：

1. 输入加载（文件/命令/文本）→ 分行/截取/换行
2. 语法高亮（syntect）或 ANSI 解析 → 生成带样式的行数据
3. 生成 SVG（包含字体 CSS、fallback family、标题栏等）
4. SVG → 栅格化（usvg parse + fontdb + resvg render）→ Pixmap
5. Pixmap → PNG/WebP 编码
6. PNG 可选优化（oxipng）与可选轻度有损（quantize）

现有主要性能浪费点：

- CLI `out.{png,webp}` 会对同一份 SVG 走两次“SVG→Pixmap”栅格化（PNG/WebP 各一次）。
- Core 在 rasterize 阶段每次都构建 fontdb；在多次渲染（尤其 Node/服务端）时成本累积明显。
- 多输出场景会重复做字体需求扫描与字体准备（`render_svg`/`render_png_from_svg`/`render_webp_from_svg` 各自计算）。

## 目标

- CLI 多输出：当一次命令同时需要 `png+webp` 时，只栅格化一次，复用同一张 pixmap，分别编码输出。
- Core：缓存/复用 fontdb 与字体 family 集合，提升进程内多次渲染吞吐（Node/服务端收益最大）。
- 减少重复扫描：允许上游在“渲染 SVG”阶段得到后续 raster 所需的最小计划信息（例如 `needs_system_fonts`）。
- 保持外部接口稳定：CLI/Node 对外参数不变；新增 Core API 以支持复用与缓存。

## 非目标 / 约束

- 不引入新的输出格式或新的 CLI flag（保持行为简单）。
- 不做跨进程缓存（仅进程内缓存）。
- 不保证 `rsvg-convert` 与 `resvg` 的像素输出完全一致。

## 关键决策

### 多格式输出强制使用 resvg（速度优先）

当一次输出同时包含 `png+webp` 时：

- 由于 WebP 仅支持 resvg 后端（当前 rsvg 后端不支持 webp），且我们要复用栅格化结果，
- 因此该分支 **强制走 resvg**，并且只 rasterize 一次。

这会带来一个可接受的变化：在安装了 `rsvg-convert` 的机器上，纯 PNG 默认可能走 `rsvg-convert`（Auto），但 `png+webp` 同次输出会强制走 resvg，像素可能略有差异。

## 设计方案概览

### 1) Core：暴露 “SVG 渲染计划” 与 “一次栅格多编码” API

- 新增 `render_svg_planned(...) -> PlannedSvg`
  - 返回：`svg_bytes` + `needs_system_fonts`
  - 目的：CLI 多输出时复用字体计划，避免 `render_png_from_svg` 再做一遍字体扫描。

- 新增 `render_png_webp_from_svg_once(svg, config, needs_system_fonts) -> {png, webp}`
  - 内部只做一次 `rasterize_svg(...) -> Pixmap`
  - 复用 Pixmap 分别编码 png/webp

### 2) Core：字体相关缓存（提升 Node/服务端吞吐）

- 缓存 `build_fontdb(config, needs_system_fonts)` 的结果
  - Key：`resolved_font_dirs + font_file(path+mtime/len) + needs_system_fonts`
  - Value：`usvg::fontdb::Database`（可 Clone）
  - 策略：小容量（例如 4～8 条）LRU/近似 LRU，避免内存膨胀

- 缓存字体 family 集合
  - `load_system_font_families()`：全局 Lazy 一次性缓存
  - `load_app_font_families(config)`：按 `resolved_font_dirs` 做小容量缓存

- 缓存失效
  - 当 `ensure_fonts_available` 实际下载/写入字体文件时，清空 app/fontdb 相关缓存，确保下一次渲染能拾取新字体。

### 3) CLI：多输出路径改为复用计划与（必要时）复用栅格化

- 仍先渲染一次 SVG（因为输出可能含 `.svg`）
- 使用 Core 的 planned API 拿到 `needs_system_fonts`
- 若同时需要 `png+webp`：
  - 复制一份 config，将 `raster.backend` 强制设置为 `resvg`
  - 调用 “一次栅格多编码” API 生成两份 bytes
- 其他组合（`svg+png`、`svg+webp`）：
  - 使用 planned API 的 `needs_system_fonts` 走单格式渲染，避免重复扫描

### 4) Node：保持 JS API 不变，增加轻量缓存

- 对 `config_json -> Config` 做小容量缓存（按字符串 hash key），减少高频调用时的 JSON 解析开销。
- 主要吞吐收益来自 Core 的 fontdb/app families/system families 缓存。

## 测试与验收

- 单测覆盖：
  - planned API 返回的 svg 以 `<svg` 开头，且 `needs_system_fonts` 可用
  - “一次栅格多编码” API：PNG 以 `\x89PNG` 开头、WebP 以 `RIFF` 开头
  - CLI `out.{png,webp}` 能写出两个文件（并间接验证不重复 rasterize 的路径可走通）
- 门禁：
  - `cargo fmt --check`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo test --workspace`
  - `cargo llvm-cov ... --fail-under-lines 80`（保持现有覆盖率门槛）

