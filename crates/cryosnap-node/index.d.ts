export type BoxValue = number | number[] | string;
export type LinesValue = number | number[] | string;

export interface BorderConfig {
  radius?: number;
  width?: number;
  color?: string;
}

export interface ShadowConfig {
  blur?: number;
  x?: number;
  y?: number;
}

export interface FontConfig {
  family?: string;
  file?: string;
  filePath?: string;
  size?: number;
  ligatures?: boolean;
}

export interface PngConfig {
  optimize?: boolean;
  level?: number;
  optimizeLevel?: number;
  strip?: 'none' | 'safe' | 'all';
  quantize?: boolean;
  quantize_preset?: 'fast' | 'balanced' | 'best';
  quantizePreset?: 'fast' | 'balanced' | 'best';
  quantize_quality?: number;
  quantizeQuality?: number;
  quantize_speed?: number;
  quantizeSpeed?: number;
  quantize_dither?: number;
  quantizeDither?: number;
}

export interface RasterConfig {
  scale?: number;
  max_pixels?: number;
  maxPixels?: number;
  backend?: 'auto' | 'resvg' | 'rsvg';
}

export interface TitleConfig {
  enabled?: boolean;
  text?: string;
  path_style?: 'absolute' | 'relative' | 'basename';
  pathStyle?: 'absolute' | 'relative' | 'basename';
  tmux_format?: string;
  tmuxFormat?: string;
  align?: 'left' | 'center' | 'right';
  size?: number;
  color?: string;
  opacity?: number;
  max_width?: number;
  maxWidth?: number;
  ellipsis?: string;
}

export interface RenderConfig {
  theme?: string;
  background?: string;
  padding?: BoxValue;
  margin?: BoxValue;
  width?: number;
  height?: number;
  window?: boolean;
  windowControls?: boolean;
  show_line_numbers?: boolean;
  showLineNumbers?: boolean;
  language?: string;
  execute_timeout_ms?: number;
  executeTimeoutMs?: number;
  wrap?: number;
  lines?: LinesValue;
  border?: BorderConfig;
  shadow?: ShadowConfig;
  font?: FontConfig;
  raster?: RasterConfig;
  png?: PngConfig;
  title?: TitleConfig;
  line_height?: number;
  lineHeight?: number;
}

export interface RenderOptions {
  input: string;
  inputKind?: 'text' | 'file' | 'command';
  configJson?: string;
  config?: RenderConfig;
  format?: 'svg' | 'png' | 'webp';
}

export function render(options: RenderOptions): Buffer;
export function renderSvg(options: RenderOptions): Buffer;
export function renderPng(options: RenderOptions): Buffer;
export function renderWebp(options: RenderOptions): Buffer;
export function renderToFile(options: RenderOptions, outputPath: string): string;
export function version(): string;
