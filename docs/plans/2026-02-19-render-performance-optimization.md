# Render Performance Optimization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 提升 Cryosnap 在 CLI 多输出与 Node/服务端高频渲染场景的吞吐：减少重复 rasterize、缓存字体数据库与字体家族集合、减少重复的计划/解析开销。

**Architecture:** Core 新增 planned SVG 与“一次栅格多编码”API；字体相关数据采用进程内小容量缓存并在字体下载落盘后失效；CLI 在 `out.{...}` 分支复用 planned 与 pixmap；Node 侧缓存 `config_json -> Config` 解析结果。

**Tech Stack:** Rust（cryosnap-core/cli/node）、usvg/resvg/tiny-skia、syntect、once_cell、Mutex/Lazy。

---

### Task 1: Core planned SVG API（复用字体计划）

**Files:**
- Modify: `crates/cryosnap-core/src/render.rs`
- Modify: `crates/cryosnap-core/src/lib.rs`
- Test: `crates/cryosnap-core/src/tests/mod.rs`

**Step 1: 写失败测试（新 API 存在且可用）**

在 `crates/cryosnap-core/src/tests/mod.rs` 增加：
- `render_svg_planned_basic_returns_svg_and_plan()`
  - `let planned = cryosnap_core::render_svg_planned(&InputSource::Text("hello".into()), &Config::default())?;`
  - 断言 `planned.bytes.starts_with(b"<svg")`

**Step 2: 跑测试确认失败**

Run: `cargo test -p cryosnap-core render_svg_planned_basic -- -q`
Expected: FAIL（找不到函数/类型）

**Step 3: 最小实现**

在 `crates/cryosnap-core/src/render.rs`：
- 新增 `pub struct PlannedSvg { pub bytes: Vec<u8>, pub needs_system_fonts: bool }`
- 新增 `pub fn render_svg_planned(input: &InputSource, config: &Config) -> Result<PlannedSvg>`
  - 内部复用现有 `render_svg_with_plan` 逻辑
  - 只向外暴露 `needs_system_fonts`
- 调整 `render_svg(...)` 复用 `render_svg_planned(...).bytes`

在 `crates/cryosnap-core/src/lib.rs` 导出新函数/类型（如需）。

**Step 4: 跑测试确认通过**

Run: `cargo test -p cryosnap-core render_svg_planned_basic -- -q`
Expected: PASS

**Step 5: Commit**

Run:
- `git add crates/cryosnap-core/src/render.rs crates/cryosnap-core/src/lib.rs crates/cryosnap-core/src/tests/mod.rs`
- `git commit -m "feat(core): add planned svg render API"`

---

### Task 2: Core 字体缓存（fontdb / app families / system families）

**Files:**
- Modify: `crates/cryosnap-core/src/fonts/system.rs`
- Modify: `crates/cryosnap-core/src/fonts/download.rs`
- (Optional) Create: `crates/cryosnap-core/src/fonts/cache.rs`
- Test: `crates/cryosnap-core/src/tests/mod.rs`

**Step 1: 写失败测试（缓存可用且可失效）**

新增测试建议（避免引入时间依赖，尽量做行为验证）：
- `fontdb_cache_invalidate_after_download()`：在测试里模拟一次“下载发生”后缓存被清空（通过调用 invalidate API 并验证后续获取会重建/不同实例）。

如果不方便做强验证，可降级为：
- `fontdb_cache_smoke_test()`：连续调用两次缓存入口，确保不 panic 且返回一致的 families 集合。

**Step 2: 跑测试确认失败**

Run: `cargo test -p cryosnap-core fontdb_cache -- -q`
Expected: FAIL（缺少缓存/失效接口）

**Step 3: 最小实现（小容量缓存 + 失效）**

实现要点：
- 为 `build_fontdb` 增加缓存：
  - Key：`resolved_font_dirs` + `needs_system_fonts` + `font.file(path, mtime, len)`
  - Value：`usvg::fontdb::Database`（Clone）
  - 容量：8（超过则淘汰最旧）
- 为 `load_app_font_families` 增加缓存（Key = resolved dirs，容量 8）
- 为 `load_system_font_families` 增加 Lazy 一次性缓存
- 增加 `invalidate_font_caches()`（或等价函数），用于在字体下载落盘后清空 app/fontdb 缓存
- 在 `ensure_fonts_available` 中：仅当实际写入/下载发生时调用 invalidate

**Step 4: 跑测试确认通过**

Run: `cargo test -p cryosnap-core fontdb_cache -- -q`
Expected: PASS

**Step 5: Commit**

Run:
- `git add crates/cryosnap-core/src/fonts/* crates/cryosnap-core/src/tests/mod.rs`
- `git commit -m "perf(fonts): cache fontdb and font families"`

---

### Task 3: Core “一次栅格多编码” API（png+webp）

**Files:**
- Modify: `crates/cryosnap-core/src/render.rs`
- Modify: `crates/cryosnap-core/src/lib.rs`
- Test: `crates/cryosnap-core/src/tests/mod.rs`

**Step 1: 写失败测试**

新增测试：
- `render_png_webp_from_svg_once_basic()`
  - 先用 `render_svg_planned` 拿到 svg
  - 调用新 API 返回 `(png, webp)`
  - 断言 png header `\x89PNG`，webp header `RIFF`

**Step 2: 跑测试确认失败**

Run: `cargo test -p cryosnap-core render_png_webp_from_svg_once_basic -- -q`
Expected: FAIL（缺少 API）

**Step 3: 最小实现**

在 `crates/cryosnap-core/src/render.rs`：
- 新增 `pub fn render_png_webp_from_svg_once(svg: &[u8], config: &Config, needs_system_fonts: bool) -> Result<(Vec<u8>, Vec<u8>)>`
- 内部：
  - `let pixmap = rasterize_svg(svg, config, needs_system_fonts)?;`
  - PNG：沿用现有逻辑（quantize 可选 + optimize_png）
  - WebP：`pixmap_to_webp(&pixmap)`
- 明确要求：调用方在需要复用时应确保 backend 为 resvg（CLI 侧强制）。

**Step 4: 跑测试确认通过**

Run: `cargo test -p cryosnap-core render_png_webp_from_svg_once_basic -- -q`
Expected: PASS

**Step 5: Commit**

Run:
- `git add crates/cryosnap-core/src/render.rs crates/cryosnap-core/src/lib.rs crates/cryosnap-core/src/tests/mod.rs`
- `git commit -m "feat(core): render png+webp from one rasterization"`

---

### Task 4: CLI `out.{...}` 分支复用 planned 与 pixmap

**Files:**
- Modify: `crates/cryosnap-cli/src/run.rs`
- Test: `crates/cryosnap-cli/src/run.rs`（tests module）

**Step 1: 写失败测试**

新增测试：
- `run_with_output_pattern_writes_png_and_webp()`
  - `args.output = out.{png,webp}`
  - 输入为 stdin 文本 "hello"
  - 断言两个文件都存在

**Step 2: 跑测试确认失败**

Run: `cargo test -p cryosnap run_with_output_pattern_writes_png_and_webp -- -q`
Expected: FAIL（测试不存在/逻辑未支持）

**Step 3: 最小实现**

修改 `crates/cryosnap-cli/src/run.rs` 的 output pattern 分支：
- 替换 `render_svg(&input, &config)` 为 `render_svg_planned(...)`
- 收集 expanded outputs 中需要的格式集合
- 若同时需要 `Png` 与 `Webp`：
  - `let mut fast_cfg = config.clone(); fast_cfg.raster.backend = RasterBackend::Resvg;`
  - 调用 core 的“一次栅格多编码” API
  - 写入 `.png`/`.webp`
- 其他情况：
  - `.svg`：直接写 `planned.bytes`
  - `.png`：调用 `render_png_from_svg_with_needs(svg, needs_system_fonts)`（可新增小 helper）或复用 planned 的 `needs_system_fonts`
  - `.webp`：同理

**Step 4: 跑测试确认通过**

Run: `cargo test -p cryosnap run_with_output_pattern_writes_png_and_webp -- -q`
Expected: PASS

**Step 5: Commit**

Run:
- `git add crates/cryosnap-cli/src/run.rs`
- `git commit -m "perf(cli): avoid duplicate rasterization for multi-output"`

---

### Task 5: Node `config_json` 解析缓存

**Files:**
- Modify: `crates/cryosnap-node/src/lib.rs`
- Test: `crates/cryosnap-node/src/lib.rs`

**Step 1: 写失败测试（覆盖缓存路径）**

在 node tests 中新增：
- `render_parses_config_json_once_when_reused()`（弱验证即可）
  - 调用两次 `render`，传入相同 `config_json`
  - 断言都成功（这里主要是保障逻辑正确；性能收益靠基准测试）

**Step 2: 跑测试确认失败**

Run: `cargo test -p cryosnap-node render_parses_config_json_once_when_reused -- -q`
Expected: FAIL（测试不存在）

**Step 3: 最小实现（小容量 LRU）**

在 `crates/cryosnap-node/src/lib.rs`：
- 新增 `static CONFIG_CACHE: Lazy<Mutex<VecDeque<(String, Config)>>>`
- 查找命中则 clone Config 并将条目移到队尾
- 未命中则 parse 后插入（容量 8，超出则 pop_front）

**Step 4: 跑测试确认通过**

Run: `cargo test -p cryosnap-node render_parses_config_json_once_when_reused -- -q`
Expected: PASS

**Step 5: Commit**

Run:
- `git add crates/cryosnap-node/src/lib.rs`
- `git commit -m "perf(node): cache parsed config json"`

---

### Task 6: 语法高亮 theme 避免 clone（小优化）

**Files:**
- Modify: `crates/cryosnap-core/src/syntax.rs`
- Test: `crates/cryosnap-core/src/tests/mod.rs`（现有高亮测试应覆盖）

**Step 1: 跑相关测试（先作为保护网）**

Run: `cargo test -p cryosnap-core highlight_code_with_language_and_path -- -q`
Expected: PASS

**Step 2: 实现**

- 将 theme 选择改成借用 `&Theme`（来自静态 `ThemeSet` 或静态 `CHARM_THEME`）
- 避免 `.cloned()` 与重复构建 charm theme

**Step 3: 跑测试确认通过**

Run: `cargo test -p cryosnap-core highlight_code_with_language_and_path -- -q`
Expected: PASS

**Step 4: Commit**

Run:
- `git add crates/cryosnap-core/src/syntax.rs`
- `git commit -m "perf(syntax): avoid theme clones"`

---

### Task 7: 质量门禁（与 CI 对齐）

**Files:**
- (As changed above)

**Step 1: fmt**

Run: `cargo fmt --check`
Expected: PASS

**Step 2: clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: PASS

**Step 3: test**

Run: `cargo test --workspace`
Expected: PASS

**Step 4: coverage**

Run: `cargo llvm-cov --workspace --ignore-filename-regex \"cryosnap-node\" --fail-under-lines 80`
Expected: PASS

**Step 5: Push**

Run:
- `git push -u origin codex/perf-render-opt`

