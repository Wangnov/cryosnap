use crate::{
    Error, PngOptions, PngQuantPreset, PngStrip, Result, DEFAULT_PNG_QUANTIZE_DITHER,
    DEFAULT_PNG_QUANTIZE_QUALITY, DEFAULT_PNG_QUANTIZE_SPEED, DEFAULT_WEBP_QUALITY,
    MAX_PNG_OPT_LEVEL,
};
use std::io::Cursor;

pub(crate) fn pixmap_to_webp(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>> {
    let width = pixmap.width();
    let height = pixmap.height();
    let rgba = unpremultiply_rgba(pixmap.data());
    let encoder = webp::Encoder::from_rgba(&rgba, width, height);
    let webp = encoder.encode(DEFAULT_WEBP_QUALITY);
    Ok(webp.to_vec())
}

pub(crate) fn unpremultiply_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        let a = chunk[3] as u16;
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let r = ((chunk[0] as u16 * 255 + a / 2) / a) as u8;
        let g = ((chunk[1] as u16 * 255 + a / 2) / a) as u8;
        let b = ((chunk[2] as u16 * 255 + a / 2) / a) as u8;
        out.extend_from_slice(&[r, g, b, chunk[3]]);
    }
    out
}

pub(crate) fn quantize_pixmap_to_png(
    pixmap: &tiny_skia::Pixmap,
    config: &PngOptions,
) -> Result<Vec<u8>> {
    let rgba = unpremultiply_rgba(pixmap.data());
    quantize_rgba_to_png(&rgba, pixmap.width(), pixmap.height(), config)
}

pub(crate) fn quantize_png_bytes(png: &[u8], config: &PngOptions) -> Result<Vec<u8>> {
    let (rgba, width, height) = decode_png_rgba(png)?;
    quantize_rgba_to_png(&rgba, width, height, config)
}

pub(crate) fn decode_png_rgba(png: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|err| Error::Render(format!("png decode: {err}")))?;
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| Error::Render("png decode: missing buffer size".to_string()))?;
    let mut buf = vec![0; buffer_size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|err| Error::Render(format!("png decode: {err}")))?;
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => rgb_to_rgba(data),
        png::ColorType::GrayscaleAlpha => gray_alpha_to_rgba(data),
        png::ColorType::Grayscale => gray_to_rgba(data),
        png::ColorType::Indexed => {
            return Err(Error::Render(
                "png decode: indexed color not expanded".to_string(),
            ));
        }
    };
    Ok((rgba, info.width, info.height))
}

pub(crate) fn rgb_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 3 * 4);
    for chunk in data.chunks_exact(3) {
        out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    out
}

pub(crate) fn gray_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for &g in data {
        out.extend_from_slice(&[g, g, g, 255]);
    }
    out
}

pub(crate) fn gray_alpha_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 * 4);
    for chunk in data.chunks_exact(2) {
        let g = chunk[0];
        let a = chunk[1];
        out.extend_from_slice(&[g, g, g, a]);
    }
    out
}

#[derive(Clone, Copy)]
pub(crate) struct QuantizeSettings {
    pub(crate) quality: u8,
    pub(crate) speed: u8,
    pub(crate) dither: f32,
}

impl PngQuantPreset {
    pub(crate) fn settings(self) -> QuantizeSettings {
        match self {
            PngQuantPreset::Fast => QuantizeSettings {
                quality: 70,
                speed: 7,
                dither: 0.5,
            },
            PngQuantPreset::Balanced => QuantizeSettings {
                quality: DEFAULT_PNG_QUANTIZE_QUALITY,
                speed: DEFAULT_PNG_QUANTIZE_SPEED,
                dither: DEFAULT_PNG_QUANTIZE_DITHER,
            },
            PngQuantPreset::Best => QuantizeSettings {
                quality: 95,
                speed: 1,
                dither: 1.0,
            },
        }
    }
}

pub(crate) fn quantize_settings(config: &PngOptions) -> QuantizeSettings {
    if let Some(preset) = config.quantize_preset {
        return preset.settings();
    }
    QuantizeSettings {
        quality: config.quantize_quality,
        speed: config.quantize_speed,
        dither: config.quantize_dither,
    }
}

pub(crate) fn quantize_rgba_to_png(
    rgba: &[u8],
    width: u32,
    height: u32,
    config: &PngOptions,
) -> Result<Vec<u8>> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(Error::Render(
            "png quantize: invalid rgba buffer".to_string(),
        ));
    }
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for chunk in rgba.chunks_exact(4) {
        pixels.push(imagequant::RGBA::new(
            chunk[0], chunk[1], chunk[2], chunk[3],
        ));
    }

    let mut attr = imagequant::new();
    let settings = quantize_settings(config);
    let quality = settings.quality.min(100);
    let speed = settings.speed.clamp(1, 10);
    attr.set_quality(0, quality)
        .map_err(|err| Error::Render(format!("png quantize quality: {err:?}")))?;
    attr.set_speed(speed as i32)
        .map_err(|err| Error::Render(format!("png quantize speed: {err:?}")))?;
    let mut image = attr
        .new_image(pixels, width as usize, height as usize, 0.0)
        .map_err(|err| Error::Render(format!("png quantize image: {err:?}")))?;
    let mut result = attr
        .quantize(&mut image)
        .map_err(|err| Error::Render(format!("png quantize: {err:?}")))?;
    let dither = settings.dither.clamp(0.0, 1.0);
    result
        .set_dithering_level(dither)
        .map_err(|err| Error::Render(format!("png quantize dither: {err:?}")))?;
    let (palette, indices) = result
        .remapped(&mut image)
        .map_err(|err| Error::Render(format!("png quantize remap: {err:?}")))?;
    encode_indexed_png(&palette, &indices, width, height)
}

pub(crate) fn encode_indexed_png(
    palette: &[imagequant::RGBA],
    indices: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    if indices.len() != width as usize * height as usize {
        return Err(Error::Render(
            "png quantize: invalid index buffer".to_string(),
        ));
    }
    let mut palette_bytes = Vec::with_capacity(palette.len() * 3);
    let mut trns = Vec::with_capacity(palette.len());
    let mut has_alpha = false;
    for color in palette {
        palette_bytes.extend_from_slice(&[color.r, color.g, color.b]);
        trns.push(color.a);
        if color.a < 255 {
            has_alpha = true;
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette_bytes);
    if has_alpha {
        encoder.set_trns(trns);
    }
    let mut writer = encoder
        .write_header()
        .map_err(|err| Error::Render(format!("png encode: {err}")))?;
    writer
        .write_image_data(indices)
        .map_err(|err| Error::Render(format!("png encode: {err}")))?;
    drop(writer);
    Ok(out)
}

pub(crate) fn optimize_png(png: Vec<u8>, config: &PngOptions) -> Result<Vec<u8>> {
    if !config.optimize {
        return Ok(png);
    }
    let level = config.level.min(MAX_PNG_OPT_LEVEL);
    let mut options = oxipng::Options::from_preset(level);
    options.strip = match config.strip {
        PngStrip::None => oxipng::StripChunks::None,
        PngStrip::Safe => oxipng::StripChunks::Safe,
        PngStrip::All => oxipng::StripChunks::All,
    };
    oxipng::optimize_from_memory(&png, &options)
        .map_err(|err| Error::Render(format!("png optimize: {err}")))
}
