# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
- PNG lossless optimization via oxipng with configurable level/strip.
- Optional PNG quantization via libimagequant (preset + quality/speed/dither controls).
- Configurable system font fallback (`font.fallbacks`, `font.system-fallback`) with auto detection.
- Built-in Symbols Nerd Font Mono (symbols-only) for Nerd Font glyphs.
- Removed bundled Hack Nerd Font asset.
- Title bar text with automatic file/tmux metadata.
- Adaptive raster scaling with max-pixel cap for performance.
- Optional rsvg-convert raster backend with auto detection.

## [0.1.0] - 2026-02-01
- Initial release: core renderer, CLI, Node/TS bindings.
- SVG/PNG/WebP output with ANSI capture, themes, and window styling.
- Interactive config, execute timeout, and output pattern expansion.
