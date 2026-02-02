const fs = require('fs');
const path = require('path');

function normalizeConfig(config) {
  if (!config || typeof config !== 'object') return config;
  const out = { ...config };

  if (out.window === undefined && out.windowControls !== undefined) {
    out.window = out.windowControls;
  }
  if (out.show_line_numbers === undefined && out.showLineNumbers !== undefined) {
    out.show_line_numbers = out.showLineNumbers;
  }
  if (out.line_height === undefined && out.lineHeight !== undefined) {
    out.line_height = out.lineHeight;
  }
  if (out.execute_timeout_ms === undefined && out.executeTimeoutMs !== undefined) {
    out.execute_timeout_ms = out.executeTimeoutMs;
  }

  if (out.font && typeof out.font === 'object') {
    const font = { ...out.font };
    if (font.filePath !== undefined && font.file === undefined) {
      font.file = font.filePath;
    }
    out.font = font;
  }

  if (out.png && typeof out.png === 'object') {
    const png = { ...out.png };
    if (png.optimizeLevel !== undefined && png.level === undefined) {
      png.level = png.optimizeLevel;
    }
    if (png.quantizeQuality !== undefined && png.quantize_quality === undefined) {
      png.quantize_quality = png.quantizeQuality;
    }
    if (png.quantizeSpeed !== undefined && png.quantize_speed === undefined) {
      png.quantize_speed = png.quantizeSpeed;
    }
    if (png.quantizeDither !== undefined && png.quantize_dither === undefined) {
      png.quantize_dither = png.quantizeDither;
    }
    if (png.quantizePreset !== undefined && png.quantize_preset === undefined) {
      png.quantize_preset = png.quantizePreset;
    }
    if (png.quantizePreset !== undefined && png.quantize === undefined) {
      png.quantize = true;
    }
    out.png = png;
  }

  if (out.raster && typeof out.raster === 'object') {
    const raster = { ...out.raster };
    if (raster.maxPixels !== undefined && raster.max_pixels === undefined) {
      raster.max_pixels = raster.maxPixels;
    }
    out.raster = raster;
  }

  if (out.title && typeof out.title === 'object') {
    const title = { ...out.title };
    if (title.pathStyle !== undefined && title.path_style === undefined) {
      title.path_style = title.pathStyle;
    }
    if (title.tmuxFormat !== undefined && title.tmux_format === undefined) {
      title.tmux_format = title.tmuxFormat;
    }
    if (title.maxWidth !== undefined && title.max_width === undefined) {
      title.max_width = title.maxWidth;
    }
    out.title = title;
  }

  return out;
}

function applyConfig(options) {
  const opts = { ...(options || {}) };
  if (opts.inputKind !== undefined && opts.input_kind === undefined) {
    opts.input_kind = opts.inputKind;
  }
  if (opts.configJson !== undefined && opts.config_json === undefined) {
    opts.config_json = opts.configJson;
  }
  if (opts.config && !opts.configJson) {
    const normalized = normalizeConfig(opts.config);
    opts.configJson = JSON.stringify(normalized);
    if (opts.config_json === undefined) {
      opts.config_json = opts.configJson;
    }
  }
  delete opts.config;
  delete opts.inputKind;
  return opts;
}

function resolveFormatFromPath(outputPath) {
  const ext = path.extname(outputPath).toLowerCase();
  if (ext === '.png') return 'png';
  if (ext === '.svg') return 'svg';
  if (ext === '.webp') return 'webp';
  return null;
}

try {
  // Built by `napi build`
  const native = require('./cryosnap.node');
  const render = (options) => native.render(applyConfig(options));
  const renderSvg = (options) => render({ ...(options || {}), format: 'svg' });
  const renderPng = (options) => render({ ...(options || {}), format: 'png' });
  const renderWebp = (options) => render({ ...(options || {}), format: 'webp' });
  const renderToFile = (options, outputPath) => {
    if (!outputPath) {
      throw new Error('outputPath is required');
    }
    const opts = { ...(options || {}) };
    if (!opts.format) {
      const inferred = resolveFormatFromPath(outputPath);
      if (inferred) opts.format = inferred;
    }
    const bytes = render(opts);
    fs.writeFileSync(outputPath, bytes);
    return outputPath;
  };

  module.exports = {
    ...native,
    render,
    renderSvg,
    renderPng,
    renderWebp,
    renderToFile
  };
} catch (err) {
  const message = [
    'Unable to load cryosnap native module.',
    'Run `npm run build` in crates/cryosnap-node first.',
    err && err.message ? `Original error: ${err.message}` : ''
  ].filter(Boolean).join('\n');
  throw new Error(message);
}
