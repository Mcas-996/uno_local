//! WezTerm-only Termwiz Surface frontend.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use termwiz::caps::Capabilities;
use termwiz::cell::{AttributeChange, Intensity};
use termwiz::color::ColorAttribute;
use termwiz::image::{ImageData, ImageDataType, TextureCoordinate};
use termwiz::input::{InputEvent, KeyCode as TwKeyCode, Modifiers};
use termwiz::surface::change::{Change, Image};
use termwiz::surface::{CursorVisibility, Position, Surface};
use termwiz::terminal::buffered::BufferedTerminal;
use termwiz::terminal::{Terminal, new_terminal};

use crate::app::App;
use crate::card_art::generate_card_art;
use crate::core::Card;
use crate::frontend::{
    FallbackReason, GraphicsBackend, GraphicsChoice, KeyCode, KeyEvent, KeyModifiers, Viewport,
};
use crate::screen::{Canvas, Style, UiColor};

pub fn run(app: &mut App) -> Result<(), String> {
    let caps = Capabilities::new_from_env().map_err(|error| error.to_string())?;
    let images_available = caps.iterm2_image();
    let terminal = new_terminal(caps).map_err(|error| error.to_string())?;
    let mut terminal = BufferedTerminal::new(terminal).map_err(|error| error.to_string())?;
    terminal
        .terminal()
        .set_raw_mode()
        .map_err(|error| error.to_string())?;
    terminal
        .terminal()
        .enter_alternate_screen()
        .map_err(|error| error.to_string())?;
    app.setup.graphics = if images_available {
        GraphicsChoice::GraphicsBeta
    } else {
        GraphicsChoice::Text
    };
    let result = run_loop(app, &mut terminal, images_available);
    let _ = terminal.add_change(Change::ClearScreen(ColorAttribute::Default));
    let _ = terminal.flush();
    let _ = terminal.terminal().exit_alternate_screen();
    let _ = terminal.terminal().set_cooked_mode();
    result
}

fn run_loop<T: Terminal>(
    app: &mut App,
    terminal: &mut BufferedTerminal<T>,
    mut images_available: bool,
) -> Result<(), String> {
    let mut previous: Option<Canvas> = None;
    let mut images = HashMap::<Card, Arc<ImageData>>::new();
    while !app.should_exit {
        terminal
            .check_for_resize()
            .map_err(|error| error.to_string())?;
        let (width, height) = terminal.dimensions();
        let backend = if app.setup.graphics == GraphicsChoice::Text {
            GraphicsBackend::Text(FallbackReason::Manual)
        } else if images_available {
            GraphicsBackend::Termwiz
        } else {
            GraphicsBackend::Text(FallbackReason::Encoding)
        };
        let viewport = Viewport {
            columns: width as u16,
            rows: height as u16,
        };
        let canvas = crate::screen::render(app, backend, viewport);
        if previous.as_ref() != Some(&canvas) {
            if apply_canvas(&mut *terminal, &canvas, &mut images).is_err() {
                images_available = false;
                images.clear();
                let text = crate::screen::render(
                    app,
                    GraphicsBackend::Text(FallbackReason::Encoding),
                    viewport,
                );
                apply_canvas(&mut *terminal, &text, &mut images)
                    .map_err(|error| error.to_string())?;
                previous = Some(text);
            } else {
                previous = Some(canvas);
            }
            terminal.flush().map_err(|error| error.to_string())?;
        }
        match terminal
            .terminal()
            .poll_input(Some(Duration::from_millis(50)))
            .map_err(|error| error.to_string())?
        {
            Some(InputEvent::Key(key)) => app.handle_key(convert_key(key), width as u16),
            Some(InputEvent::Resized { .. }) => previous = None,
            _ => {}
        }
        app.tick();
    }
    Ok(())
}

fn apply_canvas(
    terminal: &mut Surface,
    canvas: &Canvas,
    cache: &mut HashMap<Card, Arc<ImageData>>,
) -> termwiz::Result<()> {
    terminal.add_change(Change::ClearScreen(ColorAttribute::Default));
    terminal.add_change(Change::CursorVisibility(CursorVisibility::Hidden));
    let mut previous_style = None;
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let cell = *canvas.cell(x, y).expect("canvas coordinate");
            if cell.continuation {
                continue;
            }
            if previous_style != Some(cell.style) {
                add_style(terminal, cell.style);
                previous_style = Some(cell.style);
            }
            terminal.add_change(Change::CursorPosition {
                x: Position::Absolute(usize::from(x)),
                y: Position::Absolute(usize::from(y)),
            });
            terminal.add_change(Change::Text(cell.symbol.to_string()));
        }
    }
    for placement in &canvas.images {
        let data = match cache.get(&placement.card) {
            Some(data) => Arc::clone(data),
            None => {
                let mut cursor = Cursor::new(Vec::new());
                generate_card_art(placement.card)
                    .write_to(&mut cursor, image::ImageFormat::Png)
                    .map_err(|error| termwiz::format_err!("PNG encoding failed: {}", error))?;
                let data = Arc::new(ImageData::with_data(ImageDataType::EncodedFile(
                    cursor.into_inner(),
                )));
                cache.insert(placement.card, Arc::clone(&data));
                data
            }
        };
        terminal.add_change(Change::CursorPosition {
            x: Position::Absolute(usize::from(placement.rect.x)),
            y: Position::Absolute(usize::from(placement.rect.y)),
        });
        terminal.add_change(Change::Image(Image {
            width: usize::from(placement.rect.width),
            height: usize::from(placement.rect.height),
            top_left: TextureCoordinate::new_f32(0.0, 0.0),
            bottom_right: TextureCoordinate::new_f32(1.0, 1.0),
            image: data,
        }));
    }
    Ok(())
}

fn add_style(terminal: &mut Surface, style: Style) {
    terminal.add_change(Change::Attribute(AttributeChange::Foreground(tw_color(
        style.fg,
    ))));
    terminal.add_change(Change::Attribute(AttributeChange::Background(tw_color(
        style.bg,
    ))));
    terminal.add_change(Change::Attribute(AttributeChange::Intensity(
        if style.bold {
            Intensity::Bold
        } else {
            Intensity::Normal
        },
    )));
}

fn tw_color(color: UiColor) -> ColorAttribute {
    match color {
        UiColor::Default => ColorAttribute::Default,
        UiColor::Black => ColorAttribute::PaletteIndex(0),
        UiColor::Red => ColorAttribute::PaletteIndex(1),
        UiColor::Green => ColorAttribute::PaletteIndex(2),
        UiColor::Yellow => ColorAttribute::PaletteIndex(3),
        UiColor::Blue => ColorAttribute::PaletteIndex(4),
        UiColor::Magenta => ColorAttribute::PaletteIndex(5),
        UiColor::Cyan => ColorAttribute::PaletteIndex(6),
        UiColor::White => ColorAttribute::PaletteIndex(7),
        UiColor::Gray => ColorAttribute::PaletteIndex(8),
    }
}

fn convert_key(key: termwiz::input::KeyEvent) -> KeyEvent {
    let code = match key.key {
        TwKeyCode::Backspace => KeyCode::Backspace,
        TwKeyCode::Enter => KeyCode::Enter,
        TwKeyCode::LeftArrow => KeyCode::Left,
        TwKeyCode::RightArrow => KeyCode::Right,
        TwKeyCode::UpArrow => KeyCode::Up,
        TwKeyCode::DownArrow => KeyCode::Down,
        TwKeyCode::Escape => KeyCode::Esc,
        TwKeyCode::Char(value) => KeyCode::Char(value),
        _ => KeyCode::Unknown,
    };
    KeyEvent::new(
        code,
        KeyModifiers::from_flags(
            key.modifiers.contains(Modifiers::SHIFT),
            key.modifiers.contains(Modifiers::CTRL),
            key.modifiers.contains(Modifiers::ALT),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_is_kept_as_encoded_file_for_termwiz_renderer() {
        let card = Card::new(crate::core::Color::Red, crate::core::Rank::Number(1));
        let mut cursor = Cursor::new(Vec::new());
        generate_card_art(card)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        let data = ImageData::with_data(ImageDataType::EncodedFile(cursor.into_inner()));
        assert!(
            matches!(&*data.data(), ImageDataType::EncodedFile(bytes) if bytes.starts_with(b"\x89PNG"))
        );
    }

    #[test]
    fn graphical_scene_adds_encoded_images_to_the_surface() {
        let mut app =
            App::with_graphics(crate::i18n::Language::English, GraphicsChoice::GraphicsBeta);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let canvas = crate::screen::render(
            &app,
            GraphicsBackend::Termwiz,
            Viewport {
                columns: 80,
                rows: 28,
            },
        );
        assert_eq!(canvas.images.len(), 2);
        let mut surface = Surface::new(80, 28);
        apply_canvas(&mut surface, &canvas, &mut HashMap::new()).unwrap();
        let image_cells = surface
            .screen_cells()
            .into_iter()
            .flat_map(|line| line.iter())
            .filter_map(|cell| cell.attrs().images())
            .flatten()
            .collect::<Vec<_>>();
        assert!(!image_cells.is_empty());
        assert!(image_cells.iter().all(|image| matches!(&*image.image_data().data(), ImageDataType::EncodedFile(bytes) if bytes.starts_with(b"\x89PNG"))));
    }
}
