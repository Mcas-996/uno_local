//! Programmatically generated, language-neutral UNO card faces.

use image::{DynamicImage, Rgba, RgbaImage};

use crate::core::{Card, Color, Rank};

pub const CARD_WIDTH: u32 = 180;
pub const CARD_HEIGHT: u32 = 270;

const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);
const BLACK: Rgba<u8> = Rgba([18, 18, 24, 255]);
const WHITE: Rgba<u8> = Rgba([250, 246, 225, 255]);
const RED: Rgba<u8> = Rgba([218, 45, 55, 255]);
const YELLOW: Rgba<u8> = Rgba([245, 194, 38, 255]);
const GREEN: Rgba<u8> = Rgba([46, 157, 82, 255]);
const BLUE: Rgba<u8> = Rgba([46, 102, 210, 255]);

/// 生成与语言无关的固定尺寸 RGBA 牌面。
///
/// 图形运行时会按牌缓存该位图，再根据各预览区域的尺寸编码为终端协议数据。
pub fn generate_card_art(card: Card) -> DynamicImage {
    // 从透明画布开始，确保 Fit 缩放后牌角以外不会出现不透明方框。
    let mut image = RgbaImage::from_pixel(CARD_WIDTH, CARD_HEIGHT, TRANSPARENT);
    // 三层圆角矩形依次构成外框、白色内框和按牌色填充的主体。
    rounded_rect(&mut image, 0, 0, CARD_WIDTH, CARD_HEIGHT, 18, BLACK);
    rounded_rect(
        &mut image,
        6,
        6,
        CARD_WIDTH - 12,
        CARD_HEIGHT - 12,
        14,
        WHITE,
    );
    rounded_rect(
        &mut image,
        12,
        12,
        CARD_WIDTH - 24,
        CARD_HEIGHT - 24,
        11,
        card.color.map_or(BLACK, color_rgba),
    );

    // 万能牌使用四色背景和深色中央椭圆；普通牌则用白色椭圆形成对比。
    if card.is_wild() {
        wild_quadrants(&mut image);
        ellipse(&mut image, 34, 64, 112, 142, BLACK);
    } else {
        ellipse(&mut image, 34, 64, 112, 142, WHITE);
    }

    let ink = if card.is_wild() { WHITE } else { BLACK };
    // 禁止牌使用图形符号，其余牌共用字符点阵，避免依赖系统字体和语言环境。
    if card.rank == Rank::Skip {
        draw_block_symbol(&mut image, 90, 135, 43, 8, ink);
        draw_block_symbol(&mut image, 36, 42, 17, 4, WHITE);
        draw_block_symbol(&mut image, 144, 228, 17, 4, WHITE);
    } else {
        let label = rank_label(card.rank);
        let scale = if label.len() >= 3 { 10 } else { 16 };
        draw_centered_text(&mut image, label, 135, scale, ink);
        draw_text(&mut image, label, 22, 24, 5, WHITE);
        let corner_width = text_width(label, 5);
        draw_text(
            &mut image,
            label,
            CARD_WIDTH.saturating_sub(22 + corner_width),
            CARD_HEIGHT - 59,
            5,
            WHITE,
        );
    }

    DynamicImage::ImageRgba8(image)
}

fn color_rgba(color: Color) -> Rgba<u8> {
    match color {
        Color::Red => RED,
        Color::Yellow => YELLOW,
        Color::Green => GREEN,
        Color::Blue => BLUE,
    }
}

fn rank_label(rank: Rank) -> &'static str {
    match rank {
        Rank::Number(0) => "0",
        Rank::Number(1) => "1",
        Rank::Number(2) => "2",
        Rank::Number(3) => "3",
        Rank::Number(4) => "4",
        Rank::Number(5) => "5",
        Rank::Number(6) => "6",
        Rank::Number(7) => "7",
        Rank::Number(8) => "8",
        Rank::Number(9) => "9",
        Rank::Number(_) => "?",
        Rank::Skip => unreachable!("skip cards use the block symbol renderer"),
        Rank::Reverse => "R",
        Rank::DrawTwo => "+2",
        Rank::DrawEight => "+8",
        Rank::Wild => "W",
        Rank::WildDrawFour => "+4",
        Rank::WildDrawSixteen => "+16",
    }
}

fn rounded_rect(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    color: Rgba<u8>,
) {
    let right = x + width;
    let bottom = y + height;
    for py in y..bottom {
        for px in x..right {
            let dx = if px < x + radius {
                x + radius - px
            } else if px >= right - radius {
                px - (right - radius - 1)
            } else {
                0
            };
            let dy = if py < y + radius {
                y + radius - py
            } else if py >= bottom - radius {
                py - (bottom - radius - 1)
            } else {
                0
            };
            if dx == 0 || dy == 0 || dx * dx + dy * dy <= radius * radius {
                image.put_pixel(px, py, color);
            }
        }
    }
}

fn ellipse(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
    let cx = x as i64 * 2 + width as i64;
    let cy = y as i64 * 2 + height as i64;
    let rx = width as i64;
    let ry = height as i64;
    for py in y..y + height {
        for px in x..x + width {
            let dx = px as i64 * 2 + 1 - cx;
            let dy = py as i64 * 2 + 1 - cy;
            if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                image.put_pixel(px, py, color);
            }
        }
    }
}

fn wild_quadrants(image: &mut RgbaImage) {
    let regions = [
        (12, 12, 78, 123, RED),
        (90, 12, 78, 123, YELLOW),
        (12, 135, 78, 123, GREEN),
        (90, 135, 78, 123, BLUE),
    ];
    for (x, y, width, height, color) in regions {
        rounded_rect(image, x, y, width, height, 8, color);
    }
}

fn draw_block_symbol(
    image: &mut RgbaImage,
    center_x: i32,
    center_y: i32,
    radius: i32,
    stroke: i32,
    color: Rgba<u8>,
) {
    let inner_radius = radius - stroke;
    let slash_half_width = stroke / 2;
    for y in center_y - radius..=center_y + radius {
        for x in center_x - radius..=center_x + radius {
            if x < 0 || y < 0 || x >= image.width() as i32 || y >= image.height() as i32 {
                continue;
            }

            let dx = x - center_x;
            let dy = y - center_y;
            let distance_squared = dx * dx + dy * dy;
            let on_ring = distance_squared >= inner_radius * inner_radius
                && distance_squared <= radius * radius;
            let on_slash = (dy - dx) * (dy - dx) <= 2 * slash_half_width * slash_half_width
                && distance_squared <= radius * radius;
            if on_ring || on_slash {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn draw_centered_text(
    image: &mut RgbaImage,
    text: &str,
    center_y: u32,
    scale: u32,
    color: Rgba<u8>,
) {
    let width = text_width(text, scale);
    let x = (CARD_WIDTH.saturating_sub(width)) / 2;
    let height = 7 * scale;
    draw_text(
        image,
        text,
        x,
        center_y.saturating_sub(height / 2),
        scale,
        color,
    );
}

fn text_width(text: &str, scale: u32) -> u32 {
    text.chars().count() as u32 * 6 * scale - scale
}

fn draw_text(image: &mut RgbaImage, text: &str, mut x: u32, y: u32, scale: u32, color: Rgba<u8>) {
    for character in text.chars() {
        let glyph = glyph(character);
        for (row, bits) in glyph.into_iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    for py in 0..scale {
                        for px in 0..scale {
                            let target_x = x + column * scale + px;
                            let target_y = y + row as u32 * scale + py;
                            if target_x < image.width() && target_y < image.height() {
                                image.put_pixel(target_x, target_y, color);
                            }
                        }
                    }
                }
            }
        }
        x += 6 * scale;
    }
}

fn glyph(character: char) -> [u8; 7] {
    match character {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        _ => [0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0, 0b00100],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rank_generates_a_fixed_rgba_card() {
        let ranks = [
            Rank::Number(0),
            Rank::Number(9),
            Rank::Skip,
            Rank::Reverse,
            Rank::DrawTwo,
            Rank::DrawEight,
            Rank::Wild,
            Rank::WildDrawFour,
            Rank::WildDrawSixteen,
        ];
        for rank in ranks {
            let card = if matches!(
                rank,
                Rank::Wild | Rank::WildDrawFour | Rank::WildDrawSixteen
            ) {
                Card::wild(rank)
            } else {
                Card::new(Color::Red, rank)
            };
            let image = generate_card_art(card).into_rgba8();
            assert_eq!(image.dimensions(), (CARD_WIDTH, CARD_HEIGHT));
            assert_eq!(
                *image.get_pixel(150, 20),
                if card.is_wild() { YELLOW } else { RED }
            );
            assert_ne!(*image.get_pixel(90, 135), TRANSPARENT);
        }
    }

    #[test]
    fn colors_and_wild_quadrants_are_distinct() {
        for color in Color::ALL {
            let image = generate_card_art(Card::new(color, Rank::DrawEight)).into_rgba8();
            assert_eq!(*image.get_pixel(20, 20), color_rgba(color));
        }

        let wild = generate_card_art(Card::wild(Rank::WildDrawSixteen)).into_rgba8();
        assert_eq!(*wild.get_pixel(20, 20), RED);
        assert_eq!(*wild.get_pixel(150, 20), YELLOW);
        assert_eq!(*wild.get_pixel(20, 240), GREEN);
        assert_eq!(*wild.get_pixel(150, 240), BLUE);
    }

    #[test]
    fn skip_uses_block_symbols_in_the_center_and_corners() {
        let image = generate_card_art(Card::new(Color::Red, Rank::Skip)).into_rgba8();

        assert_eq!(*image.get_pixel(90, 92), BLACK);
        assert_eq!(*image.get_pixel(90, 135), BLACK);
        assert_eq!(*image.get_pixel(90, 155), WHITE);

        assert_eq!(*image.get_pixel(36, 25), WHITE);
        assert_eq!(*image.get_pixel(36, 42), WHITE);
        assert_eq!(*image.get_pixel(36, 50), RED);

        assert_eq!(*image.get_pixel(144, 211), WHITE);
        assert_eq!(*image.get_pixel(144, 228), WHITE);
        assert_eq!(*image.get_pixel(144, 236), RED);
    }
}
