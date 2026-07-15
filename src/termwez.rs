//! WezTerm-only Termwiz Surface frontend.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use termwiz::caps::{Capabilities, ProbeHints};
use termwiz::cell::{AttributeChange, Intensity};
use termwiz::color::ColorAttribute;
use termwiz::escape::osc::{
    ITermDimension, ITermFileData, ITermProprietary, OperatingSystemCommand,
};
use termwiz::image::{ImageData, ImageDataType, TextureCoordinate};
use termwiz::input::{InputEvent, KeyCode as TwKeyCode, Modifiers};
#[cfg(test)]
use termwiz::surface::Surface;
use termwiz::surface::change::{Change, Image};
use termwiz::surface::{CursorVisibility, Position};
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
    let caps = wezterm_capabilities()?;
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
    let _ = terminal.terminal().render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::CursorVisibility(CursorVisibility::Visible),
    ]);
    let _ = terminal.terminal().flush();
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
            let changes = match canvas_changes(&canvas, &mut images) {
                Ok(changes) => {
                    previous = Some(canvas);
                    changes
                }
                Err(_) => {
                    images_available = false;
                    images.clear();
                    let text = crate::screen::render(
                        app,
                        GraphicsBackend::Text(FallbackReason::Encoding),
                        viewport,
                    );
                    let changes =
                        canvas_changes(&text, &mut images).map_err(|error| error.to_string())?;
                    previous = Some(text);
                    changes
                }
            };
            let changes = stable_terminal_changes(&changes).map_err(|error| error.to_string())?;
            // Surface full repaint cannot reconstruct Change::Image. Render
            // the original changes through the VT-capable Termwiz terminal.
            // The typed OSC conversion prevents the first image from moving
            // the cursor and offsetting later image placements.
            terminal
                .terminal()
                .render(&changes)
                .map_err(|error| error.to_string())?;
            terminal
                .terminal()
                .flush()
                .map_err(|error| error.to_string())?;
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

fn wezterm_capabilities() -> Result<Capabilities, String> {
    // TERM is commonly `dumb` in native Windows WezTerm sessions. Giving
    // Termwiz a minimal xterm-compatible database both selects its VT
    // renderer and avoids its no-terminfo x/y fallback bug.
    let mut database = terminfo::Database::new();
    database
        .name("uno-wezterm")
        .description("Minimal WezTerm VT capabilities")
        .raw("cursor_address", b"\x1b[%i%p1%d;%p2%dH".as_slice());
    let database = database
        .build()
        .map_err(|_| "failed to build WezTerm capabilities".to_owned())?;
    Capabilities::new_with_hints(
        ProbeHints::new_from_env()
            .term_program(Some("WezTerm".to_owned()))
            .iterm2_image(Some(true))
            .terminfo_db(Some(database)),
    )
    .map_err(|error| error.to_string())
}

fn canvas_changes(
    canvas: &Canvas,
    cache: &mut HashMap<Card, Arc<ImageData>>,
) -> termwiz::Result<Vec<Change>> {
    let mut changes = vec![
        Change::ClearScreen(ColorAttribute::Default),
        Change::CursorVisibility(CursorVisibility::Hidden),
    ];
    let mut previous_style = None;
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let cell = *canvas.cell(x, y).expect("canvas coordinate");
            if cell.continuation {
                continue;
            }
            if previous_style != Some(cell.style) {
                add_style(&mut changes, cell.style);
                previous_style = Some(cell.style);
            }
            changes.push(Change::CursorPosition {
                x: Position::Absolute(usize::from(x)),
                y: Position::Absolute(usize::from(y)),
            });
            changes.push(Change::Text(cell.symbol.to_string()));
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
        changes.push(Change::CursorPosition {
            x: Position::Absolute(usize::from(placement.rect.x)),
            y: Position::Absolute(usize::from(placement.rect.y)),
        });
        changes.push(Change::Image(Image {
            width: usize::from(placement.rect.width),
            height: usize::from(placement.rect.height),
            top_left: TextureCoordinate::new_f32(0.0, 0.0),
            bottom_right: TextureCoordinate::new_f32(1.0, 1.0),
            image: data,
        }));
    }
    Ok(changes)
}

fn stable_terminal_changes(changes: &[Change]) -> termwiz::Result<Vec<Change>> {
    changes
        .iter()
        .map(|change| match change {
            Change::Image(image) => {
                let data = match &*image.image.data() {
                    ImageDataType::EncodedFile(data) => data.to_vec(),
                    _ => {
                        termwiz::bail!("Termwiz frontend requires encoded image data");
                    }
                };
                let osc = OperatingSystemCommand::ITermProprietary(ITermProprietary::File(
                    Box::new(ITermFileData {
                        name: None,
                        size: Some(data.len()),
                        width: ITermDimension::Cells(image.width as i64),
                        height: ITermDimension::Cells(image.height as i64),
                        preserve_aspect_ratio: true,
                        inline: true,
                        do_not_move_cursor: true,
                        data,
                    }),
                ));
                Ok(Change::Text(osc.to_string()))
            }
            _ => Ok(change.clone()),
        })
        .collect()
}

fn add_style(changes: &mut Vec<Change>, style: Style) {
    changes.push(Change::Attribute(AttributeChange::Foreground(tw_color(
        style.fg,
    ))));
    changes.push(Change::Attribute(AttributeChange::Background(tw_color(
        style.bg,
    ))));
    changes.push(Change::Attribute(AttributeChange::Intensity(
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
    use std::io::{self, Write};
    use termwiz::render::RenderTty;
    use termwiz::render::terminfo::TerminfoRenderer;

    #[derive(Default)]
    struct TestTty(Vec<u8>);

    impl Write for TestTty {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl RenderTty for TestTty {
        fn get_size_in_cells(&mut self) -> termwiz::Result<(usize, usize)> {
            Ok((80, 28))
        }
    }

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
        let changes = canvas_changes(&canvas, &mut HashMap::new()).unwrap();
        assert_eq!(
            changes
                .iter()
                .filter(|change| matches!(change, Change::Image(_)))
                .count(),
            2
        );
        let terminal_changes = stable_terminal_changes(&changes).unwrap();
        assert!(
            !terminal_changes
                .iter()
                .any(|change| matches!(change, Change::Image(_)))
        );
        assert_eq!(
            terminal_changes
                .iter()
                .filter(|change| matches!(change, Change::Text(text) if text.contains("doNotMoveCursor=1")))
                .count(),
            2
        );
        let mut surface = Surface::new(80, 28);
        surface.add_changes(changes);
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

    #[test]
    fn stable_change_stream_reaches_termwiz_image_renderer() {
        let card = Card::new(crate::core::Color::Blue, crate::core::Rank::Number(7));
        let canvas = Canvas {
            width: 1,
            height: 1,
            cells: vec![crate::screen::Cell::default()],
            images: vec![crate::screen::ImagePlacement {
                slot: crate::screen::ImageSlot::Discard,
                card,
                rect: crate::screen::Rect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            }],
        };
        let changes = canvas_changes(&canvas, &mut HashMap::new()).unwrap();
        let changes = stable_terminal_changes(&changes).unwrap();
        let caps = wezterm_capabilities().unwrap();
        let mut renderer = TerminfoRenderer::new(caps);
        let mut output = TestTty::default();
        renderer.render_to(&changes, &mut output).unwrap();
        let rendered = String::from_utf8(output.0).unwrap();
        assert!(rendered.contains("\u{1b}]1337;File="));
        assert!(rendered.contains("doNotMoveCursor=1"));
    }

    #[test]
    fn wezterm_cursor_address_keeps_rows_and_columns_in_order() {
        let caps = wezterm_capabilities().unwrap();
        let mut renderer = TerminfoRenderer::new(caps);
        let mut output = TestTty::default();
        renderer
            .render_to(
                &[Change::CursorPosition {
                    x: Position::Absolute(12),
                    y: Position::Absolute(5),
                }],
                &mut output,
            )
            .unwrap();
        assert_eq!(output.0, b"\x1b[6;13H");
    }
}
