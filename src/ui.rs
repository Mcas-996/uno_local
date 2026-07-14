//! 终端界面的布局与渲染。
//!
//! 本模块根据应用状态组合设置页、牌桌和覆盖层，并在终端尺寸或图像能力
//! 不满足要求时自动切回纯文本牌面。手牌布局同时供渲染和键盘导航使用，
//! 以保证光标移动与实际换行结果一致。

use crate::core::{Action, Color};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction as LayoutDirection, Layout, Rect};
use ratatui::style::{Color as TuiColor, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui_image::Image;

use crate::app::{App, Screen};
use crate::graphics::{GraphicsRuntime, PreviewSlot};
use crate::i18n::Message;

/// 完整界面能够正常布局的最小终端宽度。
pub const MIN_WIDTH: u16 = 70;
/// 完整界面能够正常布局的最小终端高度。
pub const MIN_HEIGHT: u16 = 22;
/// 启用图像牌面所需的最小终端高度。
pub const IMAGE_MIN_HEIGHT: u16 = 26;

/// 手牌区域的最小总高度，包含上下边框。
const MIN_HAND_HEIGHT: u16 = 5;
/// 事件日志区域的最小总高度，包含上下边框。
const MIN_LOG_HEIGHT: u16 = 3;
// 除牌桌和手牌外，标题、对手、日志下限、页脚及各区边框占用的总高度。
const FIXED_GAME_HEIGHT: u16 = 14;

/// 按当前应用状态绘制一帧界面。
///
/// 先绘制设置页或牌桌主体，再叠加帮助、结算、退出确认和选色窗口。
/// 当图像不应显示时会同步释放预览协议，避免终端残留上一帧的图像。
pub fn render(frame: &mut Frame<'_>, app: &App, graphics: &mut GraphicsRuntime) {
    let area = frame.area();
    // 尺寸不足时不再尝试压缩各分区，因为这样会让 Ratatui 区域重叠；
    // 仅保留明确的调整窗口提示，同时清除可能残留的图像。
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        graphics.suspend();
        frame.render_widget(
            Paragraph::new(app.language.text(Message::TooSmall))
                .alignment(Alignment::Center)
                .block(carnival_block(app.language.text(Message::Title)))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    // 是否显示图像既取决于终端能力，也取决于当前页面是否允许图像覆盖。
    let images_visible = should_render_images(
        area,
        app.game.is_some(),
        app.screen,
        app.pending_wild.is_some(),
        graphics.effective_backend(app.setup.graphics),
    );
    if !images_visible {
        graphics.suspend();
    }

    // game 是否存在代表已经创建对局；帮助、退出和结算仍以牌桌为背景，
    // 因此主体选择不直接依赖 screen。
    if app.game.is_some() {
        render_game(frame, app, area, graphics, images_visible);
    } else {
        render_setup(frame, app, area, graphics);
    }

    // 覆盖层最后绘制，保证边框和文字位于主体之上。
    match app.screen {
        Screen::Help => render_overlay(
            frame,
            area,
            62,
            21,
            app.language.text(Message::Help),
            app.language.text(Message::HelpBody),
        ),
        Screen::QuitConfirm => render_overlay(
            frame,
            area,
            42,
            7,
            app.language.text(Message::QuitTitle),
            app.language.text(Message::QuitBody),
        ),
        Screen::Result => {
            let state = app.game.as_ref().expect("result has game").public_state();
            let winner = state
                .winner
                .and_then(|id| {
                    state
                        .players
                        .into_iter()
                        .find(|player| player.id == id)
                        .map(|player| player.name)
                })
                .unwrap_or_default();
            render_overlay(
                frame,
                area,
                46,
                8,
                app.language.text(Message::Winner),
                &format!(
                    "[WIN] * {winner} *\n\n{}",
                    app.language.text(Message::NewMatchHint)
                ),
            );
        }
        Screen::Setup | Screen::Game => {}
    }

    // 选色不是独立 Screen，而是游戏页中的临时交互状态，所以单独叠加。
    if app.pending_wild.is_some() && app.screen == Screen::Game {
        render_color_picker(frame, app, area);
    }
}

/// 判断当前帧能否安全地绘制终端图像。
///
/// 除后端支持外，还要求正在显示无弹窗的游戏页且高度足够；宽度下限与
/// 完整 UI 一致，高度下限更高是为了容纳纵向牌面。
fn should_render_images(
    area: Rect,
    has_game: bool,
    screen: Screen,
    has_pending_wild: bool,
    backend: crate::graphics::GraphicsBackend,
) -> bool {
    // 弹窗和选色窗口会覆盖牌桌；此时禁用终端图像，避免某些协议的图像
    // 绘制层穿透普通字符组成的覆盖层。
    has_game
        && screen == Screen::Game
        && !has_pending_wild
        && area.width >= MIN_WIDTH
        && area.height >= IMAGE_MIN_HEIGHT
        && backend.supports_images()
}

/// 绘制居中的设置面板及当前终端实际采用的图形后端。
fn render_setup(frame: &mut Frame<'_>, app: &App, area: Rect, graphics: &GraphicsRuntime) {
    let outer = centered(area, 62, 18);
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(outer);
    frame.render_widget(
        Paragraph::new(app.language.text(Message::Title))
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(TuiColor::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(carnival_border()),
            ),
        rows[0],
    );

    // 数组顺序必须与 app::adjust_setup 使用的字段索引保持一致。
    let values = [
        format!(
            "{}: {}",
            app.language.text(Message::PlayerName),
            app.setup.name
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Bots),
            app.setup.bot_count
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Difficulty),
            app.language.difficulty(app.setup.difficulty)
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Deck),
            app.language.deck_variant(app.setup.deck_variant)
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Language),
            app.language.name()
        ),
        format!(
            "{}: {}",
            app.language.text(Message::Graphics),
            app.language.graphics(
                app.setup.graphics,
                graphics.effective_backend(app.setup.graphics)
            )
        ),
        app.language.text(Message::Start).to_owned(),
    ];
    let items = values.into_iter().enumerate().map(|(index, value)| {
        let prefix = if index == app.setup.selected {
            "> "
        } else {
            "  "
        };
        let style = if index == app.setup.selected {
            Style::default()
                .fg(TuiColor::Black)
                .bg(TuiColor::LightYellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(format!("{prefix}{value}")).style(style)
    });
    frame.render_widget(
        List::new(items).block(
            carnival_block(app.language.text(Message::Setup))
                .borders(Borders::LEFT | Borders::RIGHT),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(app.language.text(Message::SetupHint))
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(
                Block::default()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(carnival_border()),
            ),
        rows[2],
    );
}

/// 绘制游戏页的六个纵向区域。
///
/// `images_visible` 已由顶层入口综合页面状态、尺寸和后端能力计算，避免
/// 各子区域自行做出不一致的图像决策。
fn render_game(
    frame: &mut Frame<'_>,
    app: &App,
    area: Rect,
    graphics: &mut GraphicsRuntime,
    images_visible: bool,
) {
    let game = app.game.as_ref().expect("game view has game");
    let state = game.public_state();
    // 先计算手牌的真实换行数，再决定手牌区高度和滚动偏移。
    let (hand_lines, selected_hand_row) = hand_lines(
        app.language,
        app.human_hand().unwrap_or_default(),
        app.selected_card,
        area.width.saturating_sub(2) as usize,
    );
    // 图像牌面需要额外高度；手牌可用剩余空间增长，但必须给事件日志留位。
    let table_height = if images_visible { 9 } else { 5 };
    let hand_height = hand_height(hand_lines.len(), area.height, images_visible);
    let rows = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(table_height),
            Constraint::Length(hand_height),
            Constraint::Min(MIN_LOG_HEIGHT),
            Constraint::Length(3),
        ])
        .split(area);

    let current_name = state
        .players
        .iter()
        .find(|player| player.id == state.current_player)
        .map(|player| player.name.as_str())
        .unwrap_or("?");
    let header = format!(
        "{}  |  {}  |  {}: {}  |  {}: {}",
        app.language.text(Message::Title),
        app.language.deck_variant(game.deck_variant()),
        app.language.text(Message::Turn),
        current_name,
        app.language.text(Message::Direction),
        app.language.direction(state.direction)
    );
    frame.render_widget(
        Paragraph::new(header)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(TuiColor::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(carnival_block("* STAR TABLE *")),
        rows[0],
    );

    let opponents = state
        .players
        .iter()
        .filter(|player| player.id != app.human_id)
        .map(|player| {
            format!(
                "{}: {} {}",
                player.name,
                player.hand_len,
                app.language.text(Message::Cards)
            )
        })
        .collect::<Vec<_>>()
        .join("   |   ");
    frame.render_widget(
        Paragraph::new(opponents)
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(carnival_block(app.language.text(Message::Opponents))),
        rows[1],
    );

    if images_visible {
        render_image_table(frame, app, &state, rows[2], graphics);
    } else {
        render_text_table(frame, app, &state, rows[2]);
    }

    // 扣除上下边框后才是可显示的手牌文本行数。
    let visible_hand_rows = rows[3].height.saturating_sub(2) as usize;
    let hand_scroll = hand_scroll(selected_hand_row, visible_hand_rows);
    frame.render_widget(
        Paragraph::new(hand_lines)
            .scroll((u16::try_from(hand_scroll).unwrap_or(u16::MAX), 0))
            .block(carnival_block(app.language.text(Message::YourHand))),
        rows[3],
    );

    // 最新事件置顶，使固定高度日志区始终优先展示最近操作。
    let log_items = app
        .logs
        .iter()
        .rev()
        .map(|line| ListItem::new(line.as_str()));
    frame.render_widget(
        List::new(log_items).block(carnival_block(app.language.text(Message::EventLog))),
        rows[4],
    );

    // 命令模式独占页脚；普通模式组合当前状态与按规则动态生成的可用操作。
    let footer = if app.command_mode {
        format!(":{}", app.command)
    } else {
        let hint = game_hint(app);
        if app.status.is_empty() {
            hint
        } else {
            format!("{}  │  {hint}", app.status)
        }
    };
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(Alignment::Center)
            .style(Style::default().fg(TuiColor::LightCyan))
            .block(carnival_block(if app.command_mode {
                app.language.text(Message::Command)
            } else {
                ""
            })),
        rows[5],
    );
}

/// 将手牌转换成可渲染文本行，并返回选中牌所在的行号。
fn hand_lines(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
) -> (Vec<Line<'static>>, usize) {
    let layout = hand_layout(language, hand, selected_card, width);
    (layout.lines, layout.selected_row)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 一张手牌在响应式文本布局中的几何位置。
struct HandCardPosition {
    /// 牌在 `Game` 手牌数组中的索引，也是左右移动时使用的逻辑位置。
    index: usize,
    /// 贪心换行后所在的零基文本行。
    row: usize,
    // 使用中心横坐标的两倍值，避免为半个字符引入浮点数。
    center_twice: usize,
}

/// 渲染与键盘纵向导航共享的完整手牌布局结果。
struct HandLayout {
    /// 已应用本地化、颜色和选中样式的文本行。
    lines: Vec<Line<'static>>,
    /// 每张逻辑手牌对应的行号与水平中心。
    positions: Vec<HandCardPosition>,
    /// 当前选中牌所在行，用于计算垂直滚动。
    selected_row: usize,
}

/// 按终端字符宽度对手牌进行贪心换行。
///
/// 每个条目的宽度通过 Ratatui 的 `Span::width` 计算，因此中文牌名等宽度
/// 差异会同时反映在显示、滚动和键盘导航中。
fn hand_layout(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
) -> HandLayout {
    let mut lines = Vec::new();
    let mut positions = Vec::with_capacity(hand.len());
    let mut current_line = Vec::new();
    let mut current_width = 0;
    let mut selected_row = 0;

    for (index, card) in hand.iter().enumerate() {
        let selected = index == selected_card;
        // 序号、牌名和尾部分开着色，但整体共享同一个选中背景。
        let mut entry = vec![Span::styled(
            format!(" {}:[", index + 1),
            selected_style(Style::default().fg(TuiColor::Gray), selected),
        )];
        entry.extend(styled_card(language, *card, selected));
        entry.push(Span::styled(
            "]  ",
            selected_style(Style::default().fg(TuiColor::Gray), selected),
        ));
        let entry_width = entry.iter().map(Span::width).sum::<usize>();

        // 单张牌即使超过可用宽度也保留在当前行，确保窄窗口下仍能选中。
        if !current_line.is_empty() && current_width + entry_width > width {
            lines.push(Line::from(current_line));
            current_line = Vec::new();
            current_width = 0;
        }
        // 已提交的行数就是当前条目的行号；记录几何信息后再拼接 Span。
        let row = lines.len();
        positions.push(HandCardPosition {
            index,
            row,
            center_twice: current_width.saturating_mul(2).saturating_add(entry_width),
        });
        if selected {
            selected_row = row;
        }
        current_line.extend(entry);
        current_width += entry_width;
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    HandLayout {
        lines,
        positions,
        selected_row,
    }
}

/// 返回目标相邻行中横向位置最接近的手牌索引。
///
/// 该函数复用实际渲染的换行布局，供应用层处理上下方向键；不存在目标行时
/// 保持当前选择不变。
pub(crate) fn adjacent_hand_card(
    language: crate::i18n::Language,
    hand: &[crate::core::Card],
    selected_card: usize,
    width: usize,
    row_delta: isize,
) -> usize {
    let layout = hand_layout(language, hand, selected_card, width);
    // 手牌为空或选择索引暂时越界时不擅自跳到其他牌；应用层会在状态更新后
    // 负责收敛 selected_card。
    let Some(current) = layout
        .positions
        .iter()
        .find(|position| position.index == selected_card)
    else {
        return selected_card;
    };
    // checked_add_signed 同时处理向上越过首行和向下的无符号加法。
    let Some(target_row) = current.row.checked_add_signed(row_delta) else {
        return selected_card;
    };

    layout
        .positions
        .iter()
        .filter(|position| position.row == target_row)
        // 首要选择水平中心最近的牌；距离相同时以较小索引稳定决胜。
        .min_by_key(|position| {
            (
                position.center_twice.abs_diff(current.center_twice),
                position.index,
            )
        })
        .map_or(selected_card, |position| position.index)
}

/// 在手牌完整显示与事件日志最低高度之间分配手牌区高度。
fn hand_height(line_count: usize, area_height: u16, images_visible: bool) -> u16 {
    // 两行用于边框；即使没有手牌也维持最小区域，保持整体布局稳定。
    let desired_height = u16::try_from(line_count)
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .max(MIN_HAND_HEIGHT);
    // 图像牌桌比文字牌桌多占四行，这部分必须从手牌最大高度中扣除。
    let fixed_height = FIXED_GAME_HEIGHT + if images_visible { 4 } else { 0 };
    let max_height = area_height
        .saturating_sub(fixed_height + MIN_LOG_HEIGHT)
        .max(MIN_HAND_HEIGHT);
    desired_height.min(max_height)
}

/// 绘制任何终端都能显示的彩色文字牌桌。
fn render_text_table(
    frame: &mut Frame<'_>,
    app: &App,
    state: &crate::core::PublicGameState,
    area: Rect,
) {
    let mut discard_line = vec![Span::raw("      [ ")];
    discard_line.extend(styled_card(app.language, state.discard_top, false));
    discard_line.push(Span::raw(" ]"));
    let table = vec![
        Line::from(vec![
            Span::raw(format!("{}: ", app.language.text(Message::ActiveColor))),
            Span::styled(
                app.language.color(state.active_color),
                Style::default()
                    .fg(card_color(state.active_color))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(discard_line),
    ];
    frame.render_widget(
        Paragraph::new(table)
            .alignment(Alignment::Center)
            .block(carnival_block(app.language.text(Message::Table))),
        area,
    );
}

/// 将牌桌水平拆成“当前选择”和“弃牌堆顶”两个图像预览槽。
fn render_image_table(
    frame: &mut Frame<'_>,
    app: &App,
    state: &crate::core::PublicGameState,
    area: Rect,
    graphics: &mut GraphicsRuntime,
) {
    // 两个槽位拥有独立边框和协议缓存。使用百分比可让奇数宽度的余量由
    // Ratatui 稳定分配，而不在这里重复处理坐标取整。
    let columns = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    // 选中索引可能因刚摸牌或出牌短暂越界，使用 Option 安全退回空状态。
    let selected_card = app
        .human_hand()
        .and_then(|hand| hand.get(app.selected_card))
        .copied();

    let selected_title = selected_card.map_or_else(
        || app.language.text(Message::SelectedCard).to_owned(),
        |card| {
            format!(
                "{}: {}",
                app.language.text(Message::SelectedCard),
                app.language.card(card)
            )
        },
    );
    let selected_block = carnival_block(&selected_title);
    let selected_inner = selected_block.inner(columns[0]);
    frame.render_widget(selected_block, columns[0]);
    if let Some(card) = selected_card {
        render_card_preview(
            frame,
            graphics,
            PreviewSlot::Selected,
            card,
            selected_inner,
            app,
        );
    } else {
        // 手牌为空时立即清除对应协议，不能让上一次选中的牌继续留在终端。
        graphics.clear_slot(PreviewSlot::Selected);
        frame.render_widget(
            Paragraph::new(app.language.text(Message::NoSelectedCard)).alignment(Alignment::Center),
            selected_inner,
        );
    }

    let discard_title = format!(
        "{}: {} · {}: {}",
        app.language.text(Message::DiscardTop),
        app.language.card(state.discard_top),
        app.language.text(Message::ActiveColor),
        app.language.color(state.active_color)
    );
    let discard_block = carnival_block(&discard_title);
    let discard_inner = discard_block.inner(columns[1]);
    frame.render_widget(discard_block, columns[1]);
    render_card_preview(
        frame,
        graphics,
        PreviewSlot::Discard,
        state.discard_top,
        discard_inner,
        app,
    );
}

/// 请求给定槽位的终端协议并绘制牌面，失败时原地使用文字替代。
fn render_card_preview(
    frame: &mut Frame<'_>,
    graphics: &mut GraphicsRuntime,
    slot: PreviewSlot,
    card: crate::core::Card,
    area: Rect,
    app: &App,
) {
    let available = ratatui::layout::Size::new(area.width, area.height);
    if let Some(image_size) = graphics.fit_size(card, available) {
        let image_area = centered_image_area(area, image_size);
        if let Some(protocol) = graphics.protocol(slot, card, image_area) {
            frame.render_widget(Image::new(protocol), image_area);
            return;
        }
    }

    // 协议不可用或编码失败时仍展示可玩的文字牌面。
    frame.render_widget(
        Paragraph::new(app.language.card(card)).alignment(Alignment::Center),
        area,
    );
}

/// 将 graphics 返回的拟合图像尺寸限制并居中到面板内部。
fn centered_image_area(area: Rect, image_size: ratatui::layout::Size) -> Rect {
    // 拟合尺寸保持牌面比例且不保证填满请求区域；先限制到父区域，再用
    // 剩余空间的一半计算居中偏移。
    let width = image_size.width.min(area.width);
    let height = image_size.height.min(area.height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

/// 计算保持选中行可见所需的最小纵向滚动量。
fn hand_scroll(selected_row: usize, visible_rows: usize) -> usize {
    // 只滚动到“刚好能看见选中行”，减少上下移动时的画面跳动。
    selected_row.saturating_add(1).saturating_sub(visible_rows)
}

/// 根据规则引擎返回的合法操作生成当前玩家提示。
fn game_hint(app: &App) -> String {
    let game = app.game.as_ref().expect("game hint has game");
    let mut hints = Vec::new();

    // AI 回合不暴露人类玩家的出牌、摸牌提示，只保留帮助和退出入口。
    if game.current_player() == &app.human_id
        && let Ok(actions) = game.legal_actions(&app.human_id)
    {
        if actions
            .iter()
            .any(|action| matches!(action, Action::Play { .. }))
        {
            hints.push(app.language.text(Message::PlayHint));
        }
        if actions.iter().any(|action| matches!(action, Action::Draw)) {
            hints.push(app.language.text(Message::DrawHint));
        }
        if actions.iter().any(|action| matches!(action, Action::Pass)) {
            hints.push(app.language.text(Message::PassHint));
        }
    }

    hints.push(app.language.text(Message::GameUtilitiesHint));
    hints.join(" · ")
}

/// 绘制打出万能牌后的颜色选择弹窗。
fn render_color_picker(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let popup = centered(area, 52, 7);
    frame.render_widget(Clear, popup);
    let spans = Color::ALL
        .into_iter()
        .enumerate()
        .flat_map(|(index, color)| {
            // 所有颜色选项在同一行显示；选中项用白底强调，同时保留前景色，
            // 使用户仍能直接看到即将激活的规则颜色。
            let mut style = Style::default().fg(card_color(color));
            if index == app.selected_color {
                style = style.bg(TuiColor::White).add_modifier(Modifier::BOLD);
            }
            [
                Span::styled(format!(" {} ", app.language.color(color)), style),
                Span::raw("  "),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(spans),
            Line::from(app.language.text(Message::ColorHint)),
        ])
        .alignment(Alignment::Center)
        .block(carnival_block(app.language.text(Message::ChooseColor))),
        popup,
    );
}

/// 清空居中区域并绘制通用模态覆盖层。
///
/// `Clear` 很重要：它先擦除主体的字符单元，避免透明背景下内容穿透弹窗。
fn render_overlay(
    frame: &mut Frame<'_>,
    area: Rect,
    width: u16,
    height: u16,
    title: &str,
    body: &str,
) {
    let popup = centered(area, width, height);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false })
            .block(carnival_block(title)),
        popup,
    );
}

/// 在父区域内创建不超过其边界的居中矩形。
fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// 将游戏规则中的颜色映射为终端调色板颜色。
fn card_color(color: Color) -> TuiColor {
    match color {
        Color::Red => TuiColor::Red,
        Color::Yellow => TuiColor::Yellow,
        Color::Green => TuiColor::Green,
        Color::Blue => TuiColor::Blue,
    }
}

// ===== * CARD LIGHTS * =====

/// 生成一张文字牌面的彩色 Span。
///
/// `WildDrawSixteen` 被拆成多个 Span，以四种颜色突出其特殊牌面；其他牌
/// 使用规则颜色，普通万能牌则使用洋红色。
fn styled_card(
    language: crate::i18n::Language,
    card: crate::core::Card,
    selected: bool,
) -> Vec<Span<'static>> {
    use crate::core::Rank;

    if matches!(card.rank, Rank::WildDrawSixteen) {
        let wild = match language {
            crate::i18n::Language::English => "WILD",
            crate::i18n::Language::Chinese => "变色",
        };
        return vec![
            themed_span("< ", TuiColor::LightYellow, selected),
            themed_span(wild, TuiColor::LightRed, selected),
            themed_span(" +", TuiColor::LightYellow, selected),
            themed_span("1", TuiColor::LightGreen, selected),
            themed_span("6", TuiColor::LightBlue, selected),
            themed_span(" >", TuiColor::LightYellow, selected),
        ];
    }

    let color = match card.rank {
        Rank::DrawEight => card.color.map_or(TuiColor::LightYellow, card_color),
        _ => card.color.map_or(TuiColor::LightMagenta, card_color),
    };
    vec![themed_span(language.card(card), color, selected)]
}

/// 创建带主题前景色并可继承选中背景的牌面片段。
fn themed_span(
    content: impl Into<std::borrow::Cow<'static, str>>,
    color: TuiColor,
    selected: bool,
) -> Span<'static> {
    Span::styled(
        content,
        selected_style(
            Style::default().fg(color).add_modifier(Modifier::BOLD),
            selected,
        ),
    )
}

/// 为当前选中的完整牌条目叠加统一高亮背景。
fn selected_style(style: Style, selected: bool) -> Style {
    if selected {
        style.bg(TuiColor::White).add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

/// 返回所有主题边框共享的样式。
fn carnival_border() -> Style {
    Style::default()
        .fg(TuiColor::LightYellow)
        .add_modifier(Modifier::BOLD)
}

/// 构造统一使用圆角、亮色边框和标题样式的基础区块。
fn carnival_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(carnival_border())
        .title(title)
        .title_style(
            Style::default()
                .fg(TuiColor::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::core::{Action, Card, Rank};
    use crate::i18n::Language;

    fn contents(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw_text(terminal: &mut Terminal<TestBackend>, app: &App) {
        let mut graphics = GraphicsRuntime::text_for_tests();
        terminal
            .draw(|frame| render(frame, app, &mut graphics))
            .unwrap();
    }

    fn assert_rounded_corners(terminal: &Terminal<TestBackend>, area: Rect) {
        let buffer = terminal.backend().buffer();
        let right = area.x + area.width - 1;
        let bottom = area.y + area.height - 1;

        assert_eq!(buffer[(area.x, area.y)].symbol(), "╭");
        assert_eq!(buffer[(right, area.y)].symbol(), "╮");
        assert_eq!(buffer[(area.x, bottom)].symbol(), "╰");
        assert_eq!(buffer[(right, bottom)].symbol(), "╯");
    }

    #[test]
    fn setup_game_and_overlay_use_rounded_borders() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(9, 3, 62, 18));

        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(0, 0, 100, 3));

        app.screen = Screen::Help;
        draw_text(&mut terminal, &app);
        assert_rounded_corners(&terminal, Rect::new(19, 3, 62, 21));
    }

    #[test]
    fn setup_renders_in_chinese() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::Chinese);
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains('新'));
        assert!(screen.contains('电'));
        assert!(screen.contains('节'));
        assert!(screen.contains('日'));
        assert!(screen.contains("118"));
    }

    #[test]
    fn setup_renders_language_setting() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);

        draw_text(&mut terminal, &app);

        assert!(contents(&terminal).contains("Language: English"));
        assert!(contents(&terminal).contains("Graphics: Text"));
    }

    #[test]
    fn setup_reports_graphics_beta_backend() {
        use ratatui_image::picker::ProtocolType;

        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::with_graphics(
            Language::English,
            crate::graphics::GraphicsChoice::GraphicsBeta,
        );
        let mut graphics = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Sixel);

        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();

        assert!(contents(&terminal).contains("Graphics: Graphics (Beta) (Sixel)"));
    }

    #[test]
    fn image_mode_is_responsive_and_suspends_for_overlays_or_text() {
        use crate::graphics::{FallbackReason, GraphicsBackend};

        assert!(!should_render_images(
            Rect::new(0, 0, 70, 25),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Iterm2,
        ));
        assert!(should_render_images(
            Rect::new(0, 0, 70, 26),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Sixel,
        ));
        assert!(!should_render_images(
            Rect::new(0, 0, 100, 30),
            true,
            Screen::Help,
            false,
            GraphicsBackend::Kitty,
        ));
        assert!(!should_render_images(
            Rect::new(0, 0, 100, 30),
            true,
            Screen::Game,
            false,
            GraphicsBackend::Text(FallbackReason::Ssh),
        ));
    }

    #[test]
    fn image_previews_are_created_only_on_large_game_and_cleared_for_overlay() {
        use ratatui_image::picker::ProtocolType;

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::with_graphics(
            Language::English,
            crate::graphics::GraphicsChoice::GraphicsBeta,
        );
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let mut graphics = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);

        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Table"));

        terminal.backend_mut().resize(100, 28);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 2);
        let screen = contents(&terminal);
        assert!(screen.contains("Selected"));
        assert!(screen.contains("Discard"));

        terminal.backend_mut().resize(50, 10);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Terminal too small"));

        terminal.backend_mut().resize(100, 28);
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 2);

        app.screen = Screen::Help;
        terminal
            .draw(|frame| render(frame, &app, &mut graphics))
            .unwrap();
        assert_eq!(graphics.cached_preview_count(), 0);
        assert!(contents(&terminal).contains("Help"));
    }

    #[test]
    fn fitted_selected_and_discard_rectangles_are_centered_inside_their_panels() {
        use ratatui_image::picker::ProtocolType;

        let table = Rect::new(0, 6, 100, 9);
        let columns = Layout::default()
            .direction(LayoutDirection::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(table);
        let panels = [
            carnival_block("Selected").inner(columns[0]),
            carnival_block("Discard").inner(columns[1]),
        ];
        let cards = [
            Card::new(Color::Blue, Rank::Number(7)),
            Card::new(Color::Red, Rank::DrawTwo),
        ];
        let mut graphics = GraphicsRuntime::with_protocol_for_tests(ProtocolType::Iterm2);

        let image_rects = panels
            .into_iter()
            .zip(cards)
            .map(|(panel, card)| {
                let fitted = graphics
                    .fit_size(card, panel.as_size())
                    .expect("image backend should fit each preview");
                let image = centered_image_area(panel, fitted);
                assert!(image.x >= panel.x);
                assert!(image.y >= panel.y);
                assert!(image.right() <= panel.right());
                assert!(image.bottom() <= panel.bottom());

                let left_gap = image.x - panel.x;
                let right_gap = panel.right() - image.right();
                let top_gap = image.y - panel.y;
                let bottom_gap = panel.bottom() - image.bottom();
                assert!(left_gap.abs_diff(right_gap) <= 1);
                assert!(top_gap.abs_diff(bottom_gap) <= 1);
                image
            })
            .collect::<Vec<_>>();

        assert_ne!(image_rects[0].x, image_rects[1].x);
    }

    #[test]
    fn game_renders_without_ai_hands() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("Your hand"));
        assert!(screen.contains("AI 1: 7 cards"));
        assert!(!screen.contains("AI 1 hand"));
    }

    #[test]
    fn large_hand_wraps_every_card_and_tracks_the_selected_row() {
        let cards = (0..40)
            .map(|number| Card::new(Color::Red, Rank::Number(number % 10)))
            .collect::<Vec<_>>();

        let (lines, selected_row) = hand_lines(Language::English, &cards, 39, 68);
        let text = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(lines.len() > 3);
        assert_eq!(selected_row, lines.len() - 1);
        assert!(text.contains(" 1:["));
        assert!(text.contains(" 33:["));
        assert!(text.contains(" 40:["));
    }

    #[test]
    fn large_hand_scroll_keeps_the_selected_row_visible() {
        let visible_rows = 3_usize;
        let selected_row = 5_usize;
        let scroll = hand_scroll(selected_row, visible_rows);

        assert_eq!(scroll, 3);
        assert!(selected_row >= scroll);
        assert!(selected_row < scroll + visible_rows);
    }

    #[test]
    fn vertical_hand_navigation_chooses_the_nearest_horizontal_card() {
        let cards = (0..5)
            .map(|number| Card::new(Color::Red, Rank::Number(number)))
            .collect::<Vec<_>>();

        assert_eq!(adjacent_hand_card(Language::English, &cards, 2, 37, 1), 4);
        assert_eq!(adjacent_hand_card(Language::English, &cards, 4, 37, -1), 1);
    }

    #[test]
    fn vertical_hand_navigation_stops_at_the_first_and_last_rows() {
        let cards = (0..5)
            .map(|number| Card::new(Color::Blue, Rank::Number(number)))
            .collect::<Vec<_>>();

        assert_eq!(adjacent_hand_card(Language::English, &cards, 0, 25, -1), 0);
        assert_eq!(adjacent_hand_card(Language::English, &cards, 4, 25, 1), 4);
    }

    #[test]
    fn vertical_hand_navigation_uses_localized_card_widths() {
        let cards = (0..6)
            .map(|number| Card::new(Color::Yellow, Rank::Number(number)))
            .collect::<Vec<_>>();

        let target = adjacent_hand_card(Language::Chinese, &cards, 1, 24, 1);

        assert!(target > 1);
        let layout = hand_layout(Language::Chinese, &cards, target, 24);
        assert_eq!(layout.positions[target].row, 1);
    }

    #[test]
    fn hand_height_expands_without_hiding_the_event_log() {
        assert_eq!(hand_height(1, MIN_HEIGHT, false), MIN_HAND_HEIGHT);
        assert_eq!(hand_height(6, 28, false), 8);
        assert_eq!(hand_height(20, 28, false), 11);
        assert_eq!(hand_height(20, 28, true), 7);
    }

    #[test]
    fn fitted_images_are_centered_inside_their_panels() {
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), ratatui::layout::Size::new(6, 4)),
            Rect::new(17, 8, 6, 4)
        );
        assert_eq!(
            centered_image_area(Rect::new(10, 5, 20, 10), ratatui::layout::Size::new(30, 20)),
            Rect::new(10, 5, 20, 10)
        );
    }

    #[test]
    fn game_hint_changes_after_drawing() {
        let backend = TestBackend::new(140, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();

        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("D draw"));
        assert!(!screen.contains("P pass"));

        app.game
            .as_mut()
            .unwrap()
            .apply_action(&app.human_id, Action::Draw)
            .unwrap();
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(!screen.contains("D draw"));
        assert!(screen.contains("P pass"));
    }

    #[test]
    fn ai_turn_hides_human_action_hints() {
        let backend = TestBackend::new(140, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.bot_count = 1;
        app.start_match().unwrap();
        let game = app.game.as_mut().unwrap();
        game.apply_action(&app.human_id, Action::Draw).unwrap();
        game.apply_action(&app.human_id, Action::Pass).unwrap();

        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(!screen.contains("Enter play"));
        assert!(!screen.contains("D draw"));
        assert!(!screen.contains("P pass"));
        assert!(screen.contains("? help · Q quit"));
    }

    #[test]
    fn chinese_game_renders_localized_action_hint() {
        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::Chinese);
        app.setup.bot_count = 1;
        app.start_match().unwrap();

        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains('摸'));
        assert!(screen.contains('牌'));
        assert!(screen.contains('帮'));
        assert!(screen.contains('退'));
    }

    #[test]
    fn small_terminal_shows_resize_message() {
        let backend = TestBackend::new(50, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new(Language::English);
        draw_text(&mut terminal, &app);
        assert!(contents(&terminal).contains("Terminal too small"));
    }

    #[test]
    fn holiday_card_styles_keep_penalty_text_and_four_colors() {
        use crate::core::{Card, Rank};

        let draw_eight = styled_card(
            Language::English,
            Card::new(Color::Red, Rank::DrawEight),
            false,
        );
        assert_eq!(draw_eight.len(), 1);
        assert!(draw_eight[0].content.contains("+8"));
        assert_eq!(draw_eight[0].style.fg, Some(TuiColor::Red));

        let wild = styled_card(Language::English, Card::wild(Rank::WildDrawSixteen), false);
        let label = wild
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        let colors = wild
            .iter()
            .filter_map(|span| span.style.fg)
            .collect::<Vec<_>>();
        assert!(label.contains("WILD +16"));
        assert!(colors.contains(&TuiColor::LightRed));
        assert!(colors.contains(&TuiColor::LightYellow));
        assert!(colors.contains(&TuiColor::LightGreen));
        assert!(colors.contains(&TuiColor::LightBlue));
    }

    #[test]
    fn standard_setup_variant_is_visible() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.setup.deck_variant = crate::core::DeckVariant::Standard;
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("Standard 108"));
        assert!(screen.contains("STAR CARNIVAL"));
    }

    #[test]
    fn holiday_help_color_picker_and_result_use_themed_copy() {
        use crate::core::{Card, Rank};

        let backend = TestBackend::new(100, 28);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Language::English);
        app.screen = Screen::Help;
        draw_text(&mut terminal, &app);
        let screen = contents(&terminal);
        assert!(screen.contains("STAR CARNIVAL"));
        assert!(screen.contains("WILD +16 changes color"));

        app.start_match().unwrap();
        app.pending_wild = Some(Card::wild(Rank::WildDrawSixteen));
        draw_text(&mut terminal, &app);
        assert!(contents(&terminal).contains("Choose a color"));

        app.pending_wild = None;
        app.screen = Screen::Result;
        draw_text(&mut terminal, &app);
        assert!(contents(&terminal).contains("[WIN]"));
    }
}
