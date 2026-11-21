//! Header-only image metadata: no pixel decoding or codec dependencies.
use crate::{AltText, Error, ImageTransformation, ResolvedImage, Result};
fn invalid() -> Error {
    Error::Validation(
        "invalid or unsupported image (PNG, JPEG, GIF, BMP and TIFF are supported)".into(),
    )
}
fn u16at(b: &[u8], p: usize, le: bool) -> Result<u16> {
    let a = b.get(p..p + 2).ok_or_else(invalid)?;
    Ok(if le {
        u16::from_le_bytes([a[0], a[1]])
    } else {
        u16::from_be_bytes([a[0], a[1]])
    })
}
fn u32at(b: &[u8], p: usize, le: bool) -> Result<u32> {
    let a = b.get(p..p + 4).ok_or_else(invalid)?;
    Ok(if le {
        u32::from_le_bytes(a.try_into().unwrap())
    } else {
        u32::from_be_bytes(a.try_into().unwrap())
    })
}
fn tiff(b: &[u8]) -> Result<(u32, u32, f64, f64)> {
    let le = b.starts_with(b"II");
    let offset = u32at(b, 4, le)? as usize;
    let count = u16at(b, offset, le)? as usize;
    let mut tags = std::collections::BTreeMap::new();
    for i in 0..count {
        let p = offset + 2 + i * 12;
        let tag = u16at(b, p, le)?;
        let typ = u16at(b, p + 2, le)?;
        let val = match typ {
            3 => u16at(b, p + 8, le)? as f64,
            4 => u32at(b, p + 8, le)? as f64,
            5 => {
                let o = u32at(b, p + 8, le)? as usize;
                let den = u32at(b, o + 4, le)?;
                if den == 0 {
                    return Err(invalid());
                }
                u32at(b, o, le)? as f64 / den as f64
            }
            _ => continue,
        };
        tags.insert(tag, val);
    }
    let unit = tags.get(&296).copied().unwrap_or(2.);
    let factor = if unit == 3. { 2.54 } else { 1. };
    let dpi = |tag| {
        if unit == 1. {
            72.
        } else {
            tags.get(&tag).map(|v| (v * factor).round()).unwrap_or(72.)
        }
    };
    Ok((
        tags.get(&256).copied().unwrap_or(0.) as u32,
        tags.get(&257).copied().unwrap_or(0.) as u32,
        dpi(282),
        dpi(283),
    ))
}
fn header(b: &[u8]) -> Result<(&'static str, &'static str, u32, u32, f64, f64)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut p = 8;
        let (mut w, mut h, mut x, mut y) = (0, 0, 72., 72.);
        let mut ended = false;
        while p + 12 <= b.len() {
            let len = u32at(b, p, false)? as usize;
            let kind = b.get(p + 4..p + 8).ok_or_else(invalid)?;
            let end = p.checked_add(len + 12).ok_or_else(invalid)?;
            if end > b.len() {
                return Err(invalid());
            }
            if kind == b"IHDR" {
                w = u32at(b, p + 8, false)?;
                h = u32at(b, p + 12, false)?;
            }
            if kind == b"pHYs" && len >= 9 && b[p + 16] == 1 {
                let density = u32at(b, p + 8, false)?;
                x = if density == 0 {
                    72.
                } else {
                    (density as f64 * 0.0254).round()
                };
                let density = u32at(b, p + 12, false)?;
                y = if density == 0 {
                    72.
                } else {
                    (density as f64 * 0.0254).round()
                };
            }
            p = end;
            if kind == b"IEND" {
                ended = true;
                break;
            }
        }
        if !ended {
            return Err(invalid());
        }
        return Ok(("png", "image/png", w, h, x.max(1.), y.max(1.)));
    }
    if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        return Ok((
            "gif",
            "image/gif",
            u16at(b, 6, true)? as u32,
            u16at(b, 8, true)? as u32,
            72.,
            72.,
        ));
    }
    if b.starts_with(b"BM") {
        let w = u32at(b, 18, true)?;
        let h = (u32at(b, 22, true)? as i32).unsigned_abs();
        let ppm = |p| -> Result<f64> {
            let n = u32at(b, p, true)?;
            Ok(if n == 0 {
                96.
            } else {
                (n as f64 * 0.0254).round()
            })
        };
        return Ok(("bmp", "image/bmp", w, h, ppm(38)?, ppm(42)?));
    }
    if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
        let (w, h, x, y) = tiff(b)?;
        return Ok(("tiff", "image/tiff", w, h, x, y));
    }
    if b.starts_with(&[0xff, 0xd8]) {
        let (mut p, mut w, mut h, mut x, mut y) = (2, 0, 0, 72., 72.);
        while p + 4 <= b.len() {
            if b[p] != 0xff {
                p += 1;
                continue;
            }
            while p < b.len() && b[p] == 0xff {
                p += 1;
            }
            let marker = *b.get(p).ok_or_else(invalid)?;
            p += 1;
            if marker == 0xda || marker == 0xd9 {
                break;
            }
            if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
                continue;
            }
            let len = u16at(b, p, false)? as usize;
            if len < 2 || p + len > b.len() {
                return Err(invalid());
            }
            let seg = &b[p + 2..p + len];
            if [
                0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf,
            ]
            .contains(&marker)
            {
                h = u16at(seg, 1, false)? as u32;
                w = u16at(seg, 3, false)? as u32;
            }
            if marker == 0xe0 && seg.starts_with(b"JFIF\0") && seg.len() >= 12 {
                let unit = seg[7];
                let factor = if unit == 2 { 2.54 } else { 1. };
                if unit != 0 {
                    x = (u16at(seg, 8, false)? as f64 * factor).round();
                    y = (u16at(seg, 10, false)? as f64 * factor).round();
                }
            }
            if marker == 0xe1 && seg.starts_with(b"Exif\0\0") {
                let (_, _, dx, dy) = tiff(&seg[6..])?;
                x = dx;
                y = dy;
            }
            p += len;
        }
        return Ok(("jpg", "image/jpeg", w, h, x, y));
    }
    Err(invalid())
}
pub(crate) fn resolve(
    data: Vec<u8>,
    name: Option<String>,
    transform: Option<ImageTransformation>,
    alt_text: Option<AltText>,
) -> Result<ResolvedImage> {
    if let Some(transform) = &transform {
        transform.validate()?;
    }
    let (ext, mime, w, h, xdpi, ydpi) = header(&data)?;
    if w == 0 || h == 0 || xdpi <= 0. || ydpi <= 0. {
        return Err(invalid());
    }
    let nw = w as f64 / xdpi * 914400.;
    let nh = h as f64 / ydpi * 914400.;
    let t = transform.unwrap_or_default();
    let (width, height) = match (t.width, t.height) {
        (None, None) => (nw, nh),
        (Some(w), None) => (w * 12700., nh * (w * 12700.) / nw),
        (None, Some(h)) => (nw * (h * 12700.) / nh, h * 12700.),
        (Some(w), Some(h)) => (w * 12700., h * 12700.),
    };
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.
        || height <= 0.
        || width > i32::MAX as f64
        || height > i32::MAX as f64
    {
        return Err(Error::Validation(
            "image dimensions must be positive and fit DrawingML coordinates".into(),
        ));
    }
    if let Some(a) = &alt_text {
        for s in [&a.title, &a.name, &a.description].into_iter().flatten() {
            crate::xml::validate_text(s)?;
        }
    }
    Ok(ResolvedImage {
        data,
        extension: ext.into(),
        content_type: mime.into(),
        width: width.round() as i64,
        height: height.round() as i64,
        name: name.unwrap_or(format!("image.{ext}")),
        alt_text,
        anchor: t.anchor.map(Box::new),
        crop: t
            .crop
            .as_ref()
            .map(crate::ImageCrop::wire_values)
            .transpose()?,
        flip_horizontal: t.flip_horizontal,
        flip_vertical: t.flip_vertical,
        rotation: t
            .rotation
            .map(|degrees| (degrees * 60000.).round_ties_even() as i32),
    })
}
