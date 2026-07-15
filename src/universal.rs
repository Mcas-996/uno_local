//! Crossterm universal Text/Sixel frontend.

use std::collections::HashMap;
use std::io::{self, Read, Write, stdout};
use std::sync::mpsc;
use std::time::Duration;

use crossterm::QueueableCommand;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::style::{
    Attribute, Color as CtColor, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

use crate::app::App;
use crate::card_art::generate_card_art;
use crate::core::Card;
use crate::frontend::{
    FallbackReason, GraphicsBackend, GraphicsChoice, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    Viewport,
};
use crate::screen::{Canvas, ImagePlacement, ImageSlot, Rect, Style, UiColor};

const QUERY_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SixelCapability {
    pub cell_width: u16,
    pub cell_height: u16,
}

pub fn parse_capability_response(bytes: &[u8]) -> Option<SixelCapability> {
    let text = String::from_utf8_lossy(bytes);
    let has_sixel = text.split('c').any(|part| {
        part.rsplit_once("\x1b[?")
            .is_some_and(|(_, values)| values.split(';').any(|value| value.trim() == "4"))
    });
    let cell = text.split('t').find_map(|part| {
        let (_, values) = part.rsplit_once("\x1b[")?;
        let mut values = values.split(';');
        (values.next()? == "6").then_some(())?;
        let height = values.next()?.parse::<u16>().ok()?;
        let width = values.next()?.parse::<u16>().ok()?;
        (width > 0 && height > 0).then_some(SixelCapability {
            cell_width: width,
            cell_height: height,
        })
    });
    has_sixel.then_some(())?;
    cell
}

fn query_sixel() -> Option<SixelCapability> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut stdin = io::stdin();
        let mut chunk = [0_u8; 64];
        loop {
            match stdin.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    bytes.extend_from_slice(&chunk[..count]);
                    if bytes.windows(4).any(|window| window == b"\x1b[0n") {
                        break;
                    }
                }
            }
        }
        let _ = tx.send(bytes);
    });
    let mut out = stdout();
    out.write_all(b"\x1b[c\x1b[16t\x1b[5n").ok()?;
    out.flush().ok()?;
    rx.recv_timeout(QUERY_TIMEOUT)
        .ok()
        .and_then(|bytes| parse_capability_response(&bytes))
}

pub fn run(app: &mut App, forced_text: Option<FallbackReason>) -> io::Result<()> {
    let _guard = CrosstermGuard::enter()?;
    let capability = forced_text.is_none().then(query_sixel).flatten();
    let mut runtime = SixelRuntime::new(capability);
    if app.screen == crate::app::Screen::Setup {
        app.setup.graphics = if capability.is_some() {
            GraphicsChoice::GraphicsBeta
        } else {
            GraphicsChoice::Text
        };
    }
    let mut previous = None;
    while !app.should_exit {
        let (width, height) = crossterm::terminal::size()?;
        let backend = runtime.effective_backend(app.setup.graphics, forced_text);
        let canvas = crate::screen::render(
            app,
            backend,
            Viewport {
                columns: width,
                rows: height,
            },
        );
        runtime.draw(&mut previous, canvas)?;
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => app.handle_key(convert_key(key), width),
                Event::Resize(_, _) => {
                    previous = None;
                    runtime.invalidate();
                    execute!(stdout(), Clear(ClearType::All))?;
                }
                _ => {}
            }
        }
        app.tick();
    }
    runtime.clear_images()?;
    Ok(())
}

fn convert_key(key: crossterm::event::KeyEvent) -> KeyEvent {
    use crossterm::event::{KeyCode as C, KeyEventKind as K, KeyModifiers as M};
    let code = match key.code {
        C::Backspace => KeyCode::Backspace,
        C::Enter => KeyCode::Enter,
        C::Left => KeyCode::Left,
        C::Right => KeyCode::Right,
        C::Up => KeyCode::Up,
        C::Down => KeyCode::Down,
        C::Esc => KeyCode::Esc,
        C::Char(value) => KeyCode::Char(value),
        _ => KeyCode::Unknown,
    };
    let modifiers = KeyModifiers::from_flags(
        key.modifiers.contains(M::SHIFT),
        key.modifiers.contains(M::CONTROL),
        key.modifiers.contains(M::ALT),
    );
    let kind = match key.kind {
        K::Release => KeyEventKind::Release,
        K::Repeat => KeyEventKind::Repeat,
        _ => KeyEventKind::Press,
    };
    KeyEvent::new_with_kind(code, modifiers, kind)
}

struct CrosstermGuard;

impl CrosstermGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(stdout(), EnterAlternateScreen, Hide, Clear(ClearType::All)) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for CrosstermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Clear(ClearType::All), LeaveAlternateScreen, Show);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SixelKey {
    card: Card,
    width: u16,
    height: u16,
    cell_width: u16,
    cell_height: u16,
}

struct Emitted {
    placement: ImagePlacement,
}

struct SixelRuntime {
    capability: Option<SixelCapability>,
    cache: HashMap<SixelKey, String>,
    emitted: [Option<Emitted>; 2],
    failed: bool,
}

impl SixelRuntime {
    fn new(capability: Option<SixelCapability>) -> Self {
        Self {
            capability,
            cache: HashMap::new(),
            emitted: [None, None],
            failed: false,
        }
    }

    fn effective_backend(
        &self,
        choice: GraphicsChoice,
        forced_text: Option<FallbackReason>,
    ) -> GraphicsBackend {
        if choice == GraphicsChoice::Text {
            return GraphicsBackend::Text(FallbackReason::Manual);
        }
        if let Some(reason) = forced_text {
            return GraphicsBackend::Text(reason);
        }
        if self.failed {
            return GraphicsBackend::Text(FallbackReason::Encoding);
        }
        self.capability
            .map_or(GraphicsBackend::Text(FallbackReason::Unsupported), |_| {
                GraphicsBackend::Sixel
            })
    }

    fn invalidate(&mut self) {
        self.emitted = [None, None];
    }

    fn clear_images(&mut self) -> io::Result<()> {
        let mut out = stdout();
        for emitted in self.emitted.iter().flatten() {
            clear_rect(&mut out, emitted.placement.rect)?;
        }
        out.flush()?;
        self.invalidate();
        Ok(())
    }

    fn draw(&mut self, previous: &mut Option<Canvas>, canvas: Canvas) -> io::Result<()> {
        let mut out = stdout();
        for index in 0..2 {
            let desired = canvas
                .images
                .iter()
                .find(|item| slot_index(item.slot) == index)
                .copied();
            if self.emitted[index].as_ref().map(|old| old.placement) != desired {
                if let Some(old) = self.emitted[index].as_ref() {
                    clear_rect(&mut out, old.placement.rect)?;
                }
                self.emitted[index] = None;
            }
        }
        draw_canvas_diff(&mut out, previous.as_ref(), &canvas)?;
        if self.capability.is_some() && !self.failed {
            for placement in canvas.images.iter().copied() {
                let index = slot_index(placement.slot);
                if self.emitted[index].is_none() {
                    match self.encode(placement) {
                        Ok((key, data)) => {
                            out.write_all(
                                format!(
                                    "\x1b[s\x1b[{};{}H",
                                    placement.rect.y + 1,
                                    placement.rect.x + 1
                                )
                                .as_bytes(),
                            )?;
                            out.write_all(data.as_bytes())?;
                            out.write_all(b"\x1b[u")?;
                            let _ = key;
                            self.emitted[index] = Some(Emitted { placement });
                        }
                        Err(_) => {
                            self.failed = true;
                            self.cache.clear();
                            self.clear_images()?;
                            break;
                        }
                    }
                }
            }
        }
        out.flush()?;
        *previous = Some(canvas);
        Ok(())
    }

    fn encode(&mut self, placement: ImagePlacement) -> Result<(SixelKey, String), String> {
        let capability = self
            .capability
            .ok_or_else(|| "no Sixel capability".to_owned())?;
        let key = SixelKey {
            card: placement.card,
            width: placement.rect.width,
            height: placement.rect.height,
            cell_width: capability.cell_width,
            cell_height: capability.cell_height,
        };
        let sixel = match self.cache.entry(key) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.get().clone(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let width = u32::from(key.width) * u32::from(key.cell_width);
                let height = u32::from(key.height) * u32::from(key.cell_height);
                let image = generate_card_art(key.card)
                    .resize_exact(width, height, image::imageops::FilterType::Triangle)
                    .to_rgba8();
                let sixel = icy_sixel::SixelImage::try_from_rgba(
                    image.into_raw(),
                    width as usize,
                    height as usize,
                )
                .map_err(|error| error.to_string())?
                .encode()
                .map_err(|error| error.to_string())?;
                entry.insert(sixel).clone()
            }
        };
        Ok((key, sixel))
    }
}

fn slot_index(slot: ImageSlot) -> usize {
    match slot {
        ImageSlot::Selected => 0,
        ImageSlot::Discard => 1,
    }
}

fn clear_rect(out: &mut impl Write, rect: Rect) -> io::Result<()> {
    for row in 0..rect.height {
        out.write_all(
            format!(
                "\x1b[{};{}H\x1b[{}X",
                rect.y + row + 1,
                rect.x + 1,
                rect.width
            )
            .as_bytes(),
        )?;
    }
    Ok(())
}

fn ct_color(color: UiColor) -> CtColor {
    match color {
        UiColor::Default => CtColor::Reset,
        UiColor::Black => CtColor::Black,
        UiColor::Red => CtColor::Red,
        UiColor::Yellow => CtColor::Yellow,
        UiColor::Green => CtColor::Green,
        UiColor::Blue => CtColor::Blue,
        UiColor::Magenta => CtColor::Magenta,
        UiColor::Cyan => CtColor::Cyan,
        UiColor::White => CtColor::White,
        UiColor::Gray => CtColor::Grey,
    }
}

fn queue_style(out: &mut impl Write, style: Style) -> io::Result<()> {
    out.queue(SetForegroundColor(ct_color(style.fg)))?
        .queue(SetBackgroundColor(ct_color(style.bg)))?
        .queue(SetAttribute(if style.bold {
            Attribute::Bold
        } else {
            Attribute::NormalIntensity
        }))?;
    Ok(())
}

fn draw_canvas_diff(
    out: &mut impl Write,
    previous: Option<&Canvas>,
    current: &Canvas,
) -> io::Result<()> {
    if previous.is_none_or(|old| old.width != current.width || old.height != current.height) {
        out.queue(Clear(ClearType::All))?;
    }
    for y in 0..current.height {
        for x in 0..current.width {
            let cell = current.cell(x, y).copied().unwrap_or_default();
            if cell.continuation {
                continue;
            }
            let changed = previous
                .and_then(|old| old.cell(x, y))
                .is_none_or(|old| old != &cell);
            if changed {
                out.queue(MoveTo(x, y))?;
                queue_style(out, cell.style)?;
                out.queue(Print(cell.symbol))?;
            }
        }
    }
    out.queue(SetAttribute(Attribute::Reset))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fragmentable_sixel_and_cell_size_response() {
        assert_eq!(
            parse_capability_response(b"\x1b[?1;2;4c\x1b[6;20;10t\x1b[0n"),
            Some(SixelCapability {
                cell_width: 10,
                cell_height: 20
            })
        );
        assert_eq!(parse_capability_response(b"\x1b[?1;2c\x1b[6;20;10t"), None);
        assert_eq!(parse_capability_response(b"\x1b[?1;4c\x1b[6;0;10t"), None);
    }

    #[test]
    fn canvas_diff_emits_only_changed_cells_after_first_frame() {
        let mut first = Canvas::new(4, 2);
        first.text(0, 0, "a", Style::default());
        let mut bytes = Vec::new();
        draw_canvas_diff(&mut bytes, None, &first).unwrap();
        let initial = bytes.len();
        bytes.clear();
        draw_canvas_diff(&mut bytes, Some(&first), &first).unwrap();
        assert!(bytes.len() < initial);
    }
}
