use std::collections::HashMap;
use std::error::Error;
use cosmic_text::{
    Attrs, Buffer, CacheKey, Color as CosmicColor, FontSystem, Metrics, Shaping, SwashCache,
    SwashContent,
};
use image::imageops::FilterType;
use serenity::all::{Http, UserId};
use tiny_skia::{
    FillRule, IntSize, Paint, PathBuilder, Pattern, Pixmap, PixmapPaint, SpreadMode, Transform,
};

use crate::{get_discord_token};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct GlyphRenderKey {
    cache_key: CacheKey,
    color: CosmicColor,
    global_alpha: u8,
}

#[derive(Clone, Debug)]
struct GlyphBitmap {
    pixmap: Pixmap,
    left: i32,
    top: i32,
}

#[derive(Clone, Copy, Debug)]
struct TextBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl TextBounds {
    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

type GlyphPixmapCache = HashMap<GlyphRenderKey, GlyphBitmap>;

pub async fn get_avatar_bytes(user_id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let http = Http::new(&get_discord_token());
    let user_id = UserId::new(user_id);
    let user = user_id.to_user(&http).await?;
    let avatar_url = user.face();
    let response = reqwest::get(&avatar_url).await?;
    let response = response.error_for_status()?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

pub fn ship_image(a1_bytes: Vec<u8>, a2_bytes: Vec<u8>, p: u8) -> Option<Vec<u8>> {
    let w = 800;
    let h = 300;
    let avatar_size = 200.0;

    let mut pixmap = Pixmap::new(w, h)?;

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();
    let mut glyph_cache = GlyphPixmapCache::new();

    draw_emoji_bg(
        &mut pixmap,
        p,
        &mut font_system,
        &mut swash_cache,
        &mut glyph_cache,
        w,
        h,
    );

    draw_rounded_avatar(&mut pixmap, &a1_bytes, 50.0, 50.0, avatar_size);

    draw_rounded_avatar(&mut pixmap, &a2_bytes, w as f32 - avatar_size - 50.0, 50.0, avatar_size);

    draw_p_text(
        &mut pixmap,
        p,
        &mut font_system,
        &mut swash_cache,
        &mut glyph_cache,
        w as f32 / 2.0,
        h as f32 / 2.0,
    );

    pixmap.encode_png().ok()
}

fn draw_emoji_bg(
    pixmap: &mut Pixmap,
    p: u8,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut GlyphPixmapCache,
    w: u32,
    h: u32,
) {
    let emoji = match p {
        0..=33 => "💔",
        34..=66 => "❤️‍🩹",
        _ => "❤️",
    };
    let alpha = 80 + ((p as f32 / 100.0) * 175.0) as u8;

    let mut buffer = Buffer::new(font_system, Metrics::new(44.0, 44.0));
    {
        let mut borrowed = buffer.borrow_with(font_system);
        borrowed.set_text(
            emoji,
            &Attrs::new().color(CosmicColor::rgb(255, 255, 255)),
            Shaping::Advanced,
            None,
        );
        borrowed.shape_until_scroll(false);
    }

    let Some(bounds) = measure_text_bounds(&buffer, font_system, swash_cache) else {
        return;
    };

    let emoji_extent = bounds.width().max(bounds.height()).max(1.0);
    let positions = random_spread_points(
        15,
        w as f32,
        h as f32,
        bounds.width().max(emoji_extent * 0.8),
        bounds.height().max(emoji_extent * 0.8),
        emoji_extent * 1.18,
    );

    for (center_x, center_y) in positions {
        let draw_x = (center_x - bounds.width() / 2.0 - bounds.min_x).round();
        let draw_y = (center_y - bounds.height() / 2.0 - bounds.min_y).round();

        render_text(
            pixmap,
            &buffer,
            font_system,
            swash_cache,
            glyph_cache,
            draw_x,
            draw_y,
            CosmicColor::rgb(255, 255, 255),
            alpha,
        );
    }
}

fn draw_rounded_avatar(pixmap: &mut Pixmap, img_bytes: &[u8], x: f32, y: f32, size: f32) {
    let Ok(img) = image::load_from_memory(img_bytes) else {
        return;
    };

    let size_u32 = size.round() as u32;
    if size_u32 == 0 {
        return;
    }

    let resized = img
        .resize_exact(size_u32, size_u32, FilterType::Lanczos3)
        .to_rgba8();
    let mut rgba = resized.into_raw();
    premultiply_rgba_in_place(&mut rgba);

    let Some(int_size) = IntSize::from_wh(size_u32, size_u32) else {
        return;
    };
    let Some(avatar_pixmap) = Pixmap::from_vec(rgba, int_size) else {
        return;
    };

    let radius = size / 2.0;
    let cx = x + radius;
    let cy = y + radius;

    draw_avatar_soft_shadow(pixmap, cx, cy, radius);

    let mut avatar_path_builder = PathBuilder::new();
    avatar_path_builder.push_circle(cx, cy, radius);
    let Some(avatar_path) = avatar_path_builder.finish() else {
        return;
    };

    let mut paint = Paint::default();
    paint.shader = Pattern::new(
        avatar_pixmap.as_ref(),
        SpreadMode::Pad,
        tiny_skia::FilterQuality::Bicubic,
        1.0,
        Transform::from_translate(x, y),
    );
    paint.anti_alias = true;

    pixmap.fill_path(
        &avatar_path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn draw_p_text(
    pixmap: &mut Pixmap,
    p: u8,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut GlyphPixmapCache,
    cx: f32,
    cy: f32,
) {
    let text = format!("{p}%");
    let mut buffer = Buffer::new(font_system, Metrics::new(112.0, 112.0));
    {
        let mut borrowed = buffer.borrow_with(font_system);
        borrowed.set_text(
            &text,
            &Attrs::new(),
            Shaping::Basic,
            None,
        );
        borrowed.shape_until_scroll(false);
    }

    let Some(bounds) = measure_text_bounds(&buffer, font_system, swash_cache) else {
        return;
    };

    let x = (cx - bounds.width() / 2.0 - bounds.min_x).round();
    let y = (cy - bounds.height() / 2.0 - bounds.min_y).round();

    draw_blurred_text_outline(
        pixmap,
        &buffer,
        font_system,
        swash_cache,
        glyph_cache,
        x,
        y,
    );

    render_text(
        pixmap,
        &buffer,
        font_system,
        swash_cache,
        glyph_cache,
        x,
        y,
        CosmicColor::rgb(255, 255, 255),
        255,
    );

    let rainbow_alpha = rainbow_mask_alpha_for_percent(p);
    if rainbow_alpha > 0 {
        draw_rainbow_text_mask(
            pixmap,
            &buffer,
            font_system,
            swash_cache,
            glyph_cache,
            x,
            y,
            bounds,
            rainbow_alpha,
        );
    }
}

fn draw_avatar_soft_shadow(pixmap: &mut Pixmap, cx: f32, cy: f32, radius: f32) {
    let Some(mut shadow_layer) = Pixmap::new(pixmap.width(), pixmap.height()) else {
        return;
    };

    let mut outline_builder = PathBuilder::new();
    outline_builder.push_circle(cx, cy, radius + 5.0);
    if let Some(outline_path) = outline_builder.finish() {
        let mut outline_paint = Paint::default();
        outline_paint.set_color_rgba8(0, 0, 0, 80);
        outline_paint.anti_alias = true;
        shadow_layer.fill_path(
            &outline_path,
            &outline_paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    let mut shadow_builder = PathBuilder::new();
    shadow_builder.push_circle(cx + 4.0, cy + 4.0, radius + 2.0);
    if let Some(shadow_path) = shadow_builder.finish() {
        let mut shadow_paint = Paint::default();
        shadow_paint.set_color_rgba8(0, 0, 0, 100);
        shadow_paint.anti_alias = true;
        shadow_layer.fill_path(
            &shadow_path,
            &shadow_paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    let Some(blurred_shadow) = blur_pixmap(&shadow_layer, 4.5) else {
        return;
    };

    pixmap.draw_pixmap(
        0,
        0,
        blurred_shadow.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn draw_blurred_text_outline(
    pixmap: &mut Pixmap,
    buffer: &Buffer,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut GlyphPixmapCache,
    x: f32,
    y: f32,
) {
    let Some(mut outline_layer) = Pixmap::new(pixmap.width(), pixmap.height()) else {
        return;
    };

    let radius = 10.0;
    let samples = 16;
    for i in 0..samples {
        let angle = i as f32 / samples as f32 * std::f32::consts::PI * 2.0;
        let dx = radius * angle.cos();
        let dy = radius * angle.sin();
        render_text(
            &mut outline_layer,
            buffer,
            font_system,
            swash_cache,
            glyph_cache,
            x + dx,
            y + dy,
            CosmicColor::rgb(0, 0, 0),
            20,
        );
    }

    render_text(
        &mut outline_layer,
        buffer,
        font_system,
        swash_cache,
        glyph_cache,
        x,
        y,
        CosmicColor::rgb(0, 0, 0),
        95,
    );

    let Some(blurred_outline) = blur_pixmap(&outline_layer, 2.4) else {
        return;
    };

    pixmap.draw_pixmap(
        0,
        0,
        blurred_outline.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn draw_rainbow_text_mask(
    pixmap: &mut Pixmap,
    buffer: &Buffer,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut GlyphPixmapCache,
    x: f32,
    y: f32,
    bounds: TextBounds,
    mask_alpha: u8,
) {
    let Some(mut rainbow_layer) = Pixmap::new(pixmap.width(), pixmap.height()) else {
        return;
    };

    render_text(
        &mut rainbow_layer,
        buffer,
        font_system,
        swash_cache,
        glyph_cache,
        x,
        y,
        CosmicColor::rgb(255, 255, 255),
        255,
    );

    let text_left = x + bounds.min_x;
    let text_top = y + bounds.min_y;
    let text_width = bounds.width().max(1.0);
    let text_height = bounds.height().max(1.0);
    let layer_width = rainbow_layer.width() as usize;
    let layer_height = rainbow_layer.height() as usize;
    let pixels = rainbow_layer.data_mut();

    for py in 0..layer_height {
        for px in 0..layer_width {
            let offset = (py * layer_width + px) * 4;
            let glyph_alpha = pixels[offset + 3];
            if glyph_alpha == 0 {
                continue;
            }

            let nx = ((px as f32 - text_left) / text_width).clamp(0.0, 1.0);
            let ny = ((py as f32 - text_top) / text_height).clamp(0.0, 1.0);
            let hue = (nx * 360.0 + ny * 35.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.95, 1.0);
            let alpha = mul_u8(glyph_alpha, mask_alpha);

            pixels[offset] = premul_u8(r, alpha);
            pixels[offset + 1] = premul_u8(g, alpha);
            pixels[offset + 2] = premul_u8(b, alpha);
            pixels[offset + 3] = alpha;
        }
    }

    pixmap.draw_pixmap(
        0,
        0,
        rainbow_layer.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn blur_pixmap(pixmap: &Pixmap, sigma: f32) -> Option<Pixmap> {
    let width = pixmap.width();
    let height = pixmap.height();
    let int_size = IntSize::from_wh(width, height)?;

    if sigma <= 0.0 {
        return Pixmap::from_vec(pixmap.data().to_vec(), int_size);
    }

    let rgba = pixmap.data().to_vec();
    let image = image::RgbaImage::from_raw(width, height, rgba)?;
    let blurred = image::imageops::blur(&image, sigma);

    Pixmap::from_vec(blurred.into_raw(), int_size)
}

fn render_text(
    pixmap: &mut Pixmap,
    buffer: &Buffer,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    glyph_cache: &mut GlyphPixmapCache,
    x_offset: f32,
    y_offset: f32,
    default_color: CosmicColor,
    global_alpha: u8,
) {
    for run in buffer.layout_runs() {
        let baseline_y = y_offset + run.line_y;
        for glyph in run.glyphs {
            let physical_glyph = glyph.physical((x_offset, baseline_y), 1.0);
            let glyph_color = glyph.color_opt.unwrap_or(default_color);

            let cache_key = GlyphRenderKey {
                cache_key: physical_glyph.cache_key,
                color: glyph_color,
                global_alpha,
            };

            if !glyph_cache.contains_key(&cache_key) {
                let image_opt = swash_cache.get_image(font_system, physical_glyph.cache_key);
                if let Some(image) = image_opt.as_ref() {
                    if let Some(bitmap) = build_glyph_bitmap(image, glyph_color, global_alpha) {
                        glyph_cache.insert(cache_key, bitmap);
                    }
                }
            }

            let Some(glyph_bitmap) = glyph_cache.get(&cache_key) else {
                continue;
            };

            pixmap.draw_pixmap(
                physical_glyph.x + glyph_bitmap.left,
                physical_glyph.y - glyph_bitmap.top,
                glyph_bitmap.pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }
}

fn build_glyph_bitmap(
    image: &cosmic_text::SwashImage,
    color: CosmicColor,
    global_alpha: u8,
) -> Option<GlyphBitmap> {
    if image.placement.width == 0 || image.placement.height == 0 {
        return None;
    }

    let mut glyph_pixmap = Pixmap::new(image.placement.width, image.placement.height)?;
    let pixels = glyph_pixmap.data_mut();
    let [r, g, b, a] = color.as_rgba();

    match image.content {
        SwashContent::Mask => {
            for (i, mask_alpha) in image.data.iter().copied().enumerate() {
                let alpha = mul_u8(mul_u8(mask_alpha, a), global_alpha);
                let offset = i * 4;
                pixels[offset] = premul_u8(r, alpha);
                pixels[offset + 1] = premul_u8(g, alpha);
                pixels[offset + 2] = premul_u8(b, alpha);
                pixels[offset + 3] = alpha;
            }
        }
        SwashContent::Color => {
            for (i, rgba) in image.data.chunks_exact(4).enumerate() {
                let alpha = mul_u8(rgba[3], global_alpha);
                let offset = i * 4;
                pixels[offset] = premul_u8(rgba[0], alpha);
                pixels[offset + 1] = premul_u8(rgba[1], alpha);
                pixels[offset + 2] = premul_u8(rgba[2], alpha);
                pixels[offset + 3] = alpha;
            }
        }
        SwashContent::SubpixelMask => {
            for (i, rgb_mask) in image.data.chunks_exact(3).enumerate() {
                let coverage =
                    ((rgb_mask[0] as u16 + rgb_mask[1] as u16 + rgb_mask[2] as u16) / 3) as u8;
                let alpha = mul_u8(mul_u8(coverage, a), global_alpha);
                let offset = i * 4;
                pixels[offset] = premul_u8(r, alpha);
                pixels[offset + 1] = premul_u8(g, alpha);
                pixels[offset + 2] = premul_u8(b, alpha);
                pixels[offset + 3] = alpha;
            }
        }
    }

    Some(GlyphBitmap {
        pixmap: glyph_pixmap,
        left: image.placement.left,
        top: image.placement.top,
    })
}

fn measure_text_bounds(
    buffer: &Buffer,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
) -> Option<TextBounds> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            let physical_glyph = glyph.physical((0.0, run.line_y), 1.0);
            let Some(image) = swash_cache
                .get_image(font_system, physical_glyph.cache_key)
                .as_ref()
            else {
                continue;
            };

            let x = (physical_glyph.x + image.placement.left) as f32;
            let y = (physical_glyph.y - image.placement.top) as f32;
            let right = x + image.placement.width as f32;
            let bottom = y + image.placement.height as f32;

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(right);
            max_y = max_y.max(bottom);
        }
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some(TextBounds {
            min_x,
            min_y,
            max_x,
            max_y,
        })
    } else {
        None
    }
}

fn random_spread_points(
    total: usize,
    width: f32,
    height: f32,
    margin_x: f32,
    margin_y: f32,
    min_distance: f32,
) -> Vec<(f32, f32)> {
    if total == 0 || width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let min_x = margin_x.clamp(0.0, width);
    let max_x = (width - margin_x).clamp(0.0, width);
    let min_y = margin_y.clamp(0.0, height);
    let max_y = (height - margin_y).clamp(0.0, height);

    if min_x > max_x || min_y > max_y {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(total);
    let mut required_distance = min_distance.max(1.0);
    for _ in 0..total {
        let mut selected = None;

        for _ in 0..450 {
            let candidate = random_point_in_bounds(min_x, max_x, min_y, max_y);
            if is_far_enough(candidate, &points, required_distance) {
                selected = Some(candidate);
                break;
            }
        }

        if selected.is_none() {
            let mut best_candidate = None;
            let mut best_distance = f32::NEG_INFINITY;

            for _ in 0..120 {
                let candidate = random_point_in_bounds(min_x, max_x, min_y, max_y);
                let nearest = nearest_distance(candidate, &points);
                if nearest > best_distance {
                    best_distance = nearest;
                    best_candidate = Some(candidate);
                }
            }

            if let Some(candidate) = best_candidate {
                selected = Some(candidate);
                required_distance = required_distance.min(best_distance.max(min_distance * 0.75));
            }
        }

        let Some(point) = selected else {
            break;
        };
        points.push(point);
    }

    points
}

fn random_point_in_bounds(min_x: f32, max_x: f32, min_y: f32, max_y: f32) -> (f32, f32) {
    let x = if (max_x - min_x).abs() <= f32::EPSILON {
        min_x
    } else {
        rand::random_range(min_x..max_x)
    };

    let y = if (max_y - min_y).abs() <= f32::EPSILON {
        min_y
    } else {
        rand::random_range(min_y..max_y)
    };

    (x, y)
}

fn nearest_distance(candidate: (f32, f32), points: &[(f32, f32)]) -> f32 {
    if points.is_empty() {
        return f32::INFINITY;
    }

    points
        .iter()
        .map(|(px, py)| {
            let dx = candidate.0 - *px;
            let dy = candidate.1 - *py;
            (dx * dx + dy * dy).sqrt()
        })
        .fold(f32::INFINITY, f32::min)
}

fn is_far_enough(candidate: (f32, f32), points: &[(f32, f32)], min_distance: f32) -> bool {
    nearest_distance(candidate, points) >= min_distance
}

fn premultiply_rgba_in_place(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = pixel[3];
        pixel[0] = premul_u8(pixel[0], alpha);
        pixel[1] = premul_u8(pixel[1], alpha);
        pixel[2] = premul_u8(pixel[2], alpha);
    }
}

#[inline]
fn rainbow_mask_alpha_for_percent(percent: u8) -> u8 {
    if percent <= 25 {
        return 0;
    }

    let visible_range = 100u8.saturating_sub(25);
    let progress = percent.saturating_sub(25) as f32 / visible_range as f32;
    (progress * 255.0).round() as u8
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let hh = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hh {
        h if (0.0..1.0).contains(&h) => (c, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, c, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, c, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, c),
        h if (4.0..5.0).contains(&h) => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let m = v - c;
    let r = ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    (r, g, b)
}

#[inline]
fn mul_u8(a: u8, b: u8) -> u8 {
    ((a as u16 * b as u16 + 127) / 255) as u8
}

#[inline]
fn premul_u8(color: u8, alpha: u8) -> u8 {
    mul_u8(color, alpha)
}
