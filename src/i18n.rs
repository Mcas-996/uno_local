//! * STAR CARNIVAL WORDS *
//!
//! English and Chinese labels for every table and Holiday card.

use crate::ai::Difficulty;
use crate::app::{HandFilter, PlayMode};
use crate::core::{Card, Color, DeckVariant, Direction, GameError, Rank};
use crate::frontend::{FallbackReason, GraphicsBackend, GraphicsChoice};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Chinese];

    pub fn detect() -> Self {
        Self::from_locale(sys_locale::get_locale().as_deref())
    }

    pub fn from_locale(locale: Option<&str>) -> Self {
        if locale
            .unwrap_or_default()
            .to_ascii_lowercase()
            .starts_with("zh")
        {
            Self::Chinese
        } else {
            Self::English
        }
    }

    pub fn text(self, message: Message) -> &'static str {
        let (english, chinese) = match message {
            Message::Title => ("* UNO STAR CARNIVAL *", "* UNO 星光嘉年华 *"),
            Message::Setup => ("New local match", "新建本地对局"),
            Message::Mode => ("Mode", "模式"),
            Message::PlayerOne => ("Player 1", "玩家 1"),
            Message::PlayerTwo => ("Player 2", "玩家 2"),
            Message::Bots => ("AI opponents", "电脑玩家"),
            Message::Difficulty => ("Difficulty", "难度"),
            Message::Deck => ("Deck", "牌组"),
            Message::SevenZero => ("7-0 rule", "7-0 规则"),
            Message::Enabled => ("Enabled", "开启"),
            Message::Disabled => ("Disabled", "关闭"),
            Message::Language => ("Language", "语言"),
            Message::Graphics => ("Graphics", "图像"),
            Message::Start => ("Start match", "开始游戏"),
            Message::SetupHint => (
                "Arrows/hjkl navigate · type name · Enter start · Esc quit",
                "方向键/hjkl 导航 · 输入名称 · Enter 开始 · Esc 退出",
            ),
            Message::Table => ("Table", "牌桌"),
            Message::SelectedCard => ("Selected", "已选手牌"),
            Message::DiscardTop => ("Discard", "弃牌"),
            Message::YourHand => ("Your hand", "你的手牌"),
            Message::NoMatchingCards => ("No matching cards", "没有匹配的牌"),
            Message::NoPlayablePlusBatch => (
                "No +2/+8/+16 sequence can be played",
                "没有可连续打出的 +2/+8/+16",
            ),
            Message::EventLog => ("Events", "事件"),
            Message::Turn => ("Turn", "当前回合"),
            Message::ActiveColor => ("Active color", "当前颜色"),
            Message::Direction => ("Direction", "方向"),
            Message::Cards => ("cards", "张牌"),
            Message::ChooseColor => ("Choose a color", "选择颜色"),
            Message::ChoosePlayer => ("Choose a player", "选择玩家"),
            Message::ColorHint => (
                "←/→ or h/l choose  Enter confirm  Esc cancel",
                "←/→ 或 h/l 选择  Enter 确认  Esc 取消",
            ),
            Message::Help => ("Help", "帮助"),
            Message::HelpBody => (
                "* STAR CARNIVAL *\n\nShortcuts\n  Arrows/hjkl select  Enter play\n  F filter             D draw / P pass\n  G auto +2/+8/+16     : command  Q quit\n\nHand filter\n  F cycles All, +, -, 0/7, and x!/sqrt\n  Visible cards are renumbered from 1\n\nLarge hands\n  At most 200 cards are loaded at once\n  Press Down 5 times at the bottom for the next random page\n  Press Up 5 times at the top to regenerate the previous page\n\nAuto plus\n  G plays the longest legal +2/+8/+16 sequence\n  Intermediate +16 cards bridge colors\n\n7-0 rule\n  7 swaps hands; 0 rotates hands\n\nHoliday\n  +8 matches color/rank; WILD +16 changes color\n  WILD -32/-64 discard and share large hands\n  WILD x!: next hand becomes min(x!, x^7, 1M)\n  WILD sqrt: own remaining hand becomes floor(sqrt(x))\n  Both math wilds skip the next player\n\nCommands\n  play <visible index>  draw  pass\n  help                  new   quit\n\nPress ? or Esc to return.",
                "* 星光嘉年华 *\n\n快捷键\n  方向键/hjkl 选择手牌  Enter 出牌\n  F 筛选               D 摸牌 / P 跳过\n  G 自动出 +2/+8/+16   : 输入命令  Q 退出\n\n手牌筛选\n  F 循环：全部、+、-、0/7、x!/sqrt\n  可见牌从 1 开始重新编号\n\n大量手牌\n  每次最多加载 200 张具体牌\n  在底部按 5 次下键加载下一随机页\n  在顶部按 5 次上键重新生成上一页\n\n自动出加牌\n  G 打出最长合法 +2/+8/+16 组合\n  中间的 +16 可用于切换颜色\n\n7-0 规则\n  7 交换手牌；0 轮转手牌\n\n节日牌\n  +8 匹配颜色或牌面；变色 +16 可改变颜色\n  变色 -32/-64 可处理并分发大量手牌\n  变色阶乘：下家变为 min(x!, x^7, 1M) 张\n  变色平方根：自己剩余手牌变为 floor(sqrt(x)) 张\n  两种数学牌都会跳过下家\n\n命令\n  play <可见序号>  draw  pass\n  help              new   quit\n\n按 ? 或 Esc 返回。",
            ),
            Message::QuitTitle => ("Leave match?", "退出对局？"),
            Message::QuitBody => ("Y confirm · N/Esc cancel", "Y 确认 · N/Esc 取消"),
            Message::Winner => ("Round complete", "本局结束"),
            Message::NewMatchHint => ("N new match · Q quit", "N 新游戏 · Q 退出"),
            Message::TooSmall => (
                "Terminal too small. Resize to at least 70 × 26.",
                "终端尺寸过小，请调整到至少 70 × 26。",
            ),
            Message::Thinking => ("AI is thinking…", "AI 正在思考…"),
            Message::DrewCard => ("drew a card", "摸了一张牌"),
            Message::Passed => ("passed", "跳过回合"),
            Message::Played => ("played", "打出"),
            Message::InvalidCommand => ("Invalid command", "无效命令"),
            Message::InvalidCardIndex => ("Invalid card index", "手牌序号无效"),
            Message::Easy => ("Easy", "简单"),
            Message::Normal => ("Normal", "普通"),
            Message::Hard => ("Hard", "困难"),
            Message::Extreme => ("Extreme", "最难"),
            Message::Clockwise => ("clockwise", "顺时针"),
            Message::CounterClockwise => ("counter-clockwise", "逆时针"),
        };
        match self {
            Self::English => english,
            Self::Chinese => chinese,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
        }
    }

    pub fn play_mode(self, mode: PlayMode) -> &'static str {
        match (self, mode) {
            (Self::English, PlayMode::Single) => "Single player",
            (Self::English, PlayMode::Dual) => "Two players",
            (Self::Chinese, PlayMode::Single) => "单人",
            (Self::Chinese, PlayMode::Dual) => "双人",
        }
    }

    pub fn turn_status(self, player_name: &str) -> String {
        match self {
            Self::English => format!("{player_name}'s turn"),
            Self::Chinese => format!("轮到 {player_name}"),
        }
    }

    pub fn help_body(self, mode: PlayMode) -> &'static str {
        match (self, mode) {
            (Self::English, PlayMode::Single) => self.text(Message::HelpBody),
            (Self::Chinese, PlayMode::Single) => self.text(Message::HelpBody),
            (Self::English, PlayMode::Dual) => {
                "* STAR CARNIVAL *\n\nTwo-player shortcuts\n  Left:    WASD select\n  Right:   hjkl select\n  Current: arrows select\n  Enter play   X draw   P pass\n  F filter     G auto +2/+8/+16\n  : command    Q quit\n\nLarge hands\n  Each player loads at most 200 concrete cards\n  At a boundary, press Down/Up 5 times to change random pages\n\nAuto plus\n  G plays the current player's longest legal sequence\n  Intermediate +16 cards bridge colors\n\nHand filter\n  F cycles All, +, -, 0/7, and x!/sqrt\n  Both hands use visible indices from 1\n\n7-0 rule\n  7 swaps hands; 0 rotates hands\n\nHoliday\n  +8 matches color/rank; WILD +16 changes color\n  WILD -32/-64 discard and share large hands\n  WILD x!: next hand becomes min(x!, x^7, 1M)\n  WILD sqrt: own remaining hand becomes floor(sqrt(x))\n  Both math wilds skip the next player\n\nCommands act for the current player and use visible indices.\nPress ? or Esc to return."
            }
            (Self::Chinese, PlayMode::Dual) => {
                "* 星光嘉年华 *\n\n双人快捷键\n  左侧：WASD 选择手牌\n  右侧：hjkl 选择手牌\n  当前玩家：方向键选择手牌\n  Enter 出牌  X 摸牌  P 跳过\n  F 筛选      G 自动出 +2/+8/+16\n  : 输入命令  Q 退出\n\n大量手牌\n  每位玩家最多加载 200 张具体牌\n  在边界累计按 5 次下键/上键切换随机页\n\n自动出加牌\n  G 打出当前玩家的最长合法组合\n  中间的 +16 可用于切换颜色\n\n手牌筛选\n  F 循环：全部、+、-、0/7、x!/sqrt\n  双方可见牌均从 1 开始编号\n\n7-0 规则\n  7 交换手牌；0 轮转手牌\n\n节日牌\n  +8 匹配颜色或牌面；变色 +16 可改变颜色\n  变色 -32/-64 可处理并分发大量手牌\n  变色阶乘：下家变为 min(x!, x^7, 1M) 张\n  变色平方根：自己剩余手牌变为 floor(sqrt(x)) 张\n  两种数学牌都会跳过下家\n\n命令作用于当前玩家并使用可见序号。\n按 ? 或 Esc 返回。"
            }
        }
    }

    pub fn game_hint(self, mode: PlayMode) -> &'static str {
        match (self, mode) {
            (Self::English, PlayMode::Single) => {
                "Enter play · G +2/8/16 · F filter · D draw · P pass · ? help · Q quit"
            }
            (Self::Chinese, PlayMode::Single) => {
                "G:+2/8/16 F筛选 Enter出牌 D摸牌 P跳过 ?帮助 Q退出"
            }
            (Self::English, PlayMode::Dual) => {
                "Enter play · G +2/8/16 · F filter · X draw · P pass · ? help · Q quit"
            }
            (Self::Chinese, PlayMode::Dual) => "G:+2/8/16 F筛选 Enter出牌 X摸牌 P跳过 ?帮助 Q退出",
        }
    }

    pub const fn hand_filter(self, filter: HandFilter) -> &'static str {
        match (self, filter) {
            (Self::English, HandFilter::All) => "All",
            (Self::Chinese, HandFilter::All) => "全部",
            (_, HandFilter::Positive) => "+",
            (_, HandFilter::Negative) => "-",
            (_, HandFilter::SevenZero) => "0/7",
            (_, HandFilter::Mathematical) => "x!/sqrt",
        }
    }

    pub fn hand_page_summary(
        self,
        page: usize,
        pages: usize,
        loaded: usize,
        total: usize,
    ) -> String {
        match self {
            Self::English => format!("T:{total} P:{}/{} L:{loaded}", page + 1, pages),
            Self::Chinese => format!("总:{total} 页:{}/{} 载:{loaded}", page + 1, pages),
        }
    }

    pub fn page_press_status(self, up: bool, remaining: u8) -> String {
        match (self, up) {
            (Self::English, true) => {
                format!("Press Up {remaining} more time(s) to regenerate the previous page")
            }
            (Self::English, false) => {
                format!("Press Down {remaining} more time(s) to load the next random page")
            }
            (Self::Chinese, true) => format!("再按 {remaining} 次上键重新生成上一页"),
            (Self::Chinese, false) => format!("再按 {remaining} 次下键加载下一随机页"),
        }
    }

    pub fn page_loaded_status(self, page: usize, pages: usize) -> String {
        match self {
            Self::English => format!("Loaded random hand page {}/{pages}", page + 1),
            Self::Chinese => format!("已加载随机手牌第 {}/{pages} 页", page + 1),
        }
    }

    pub fn page_edge_status(self, up: bool) -> &'static str {
        match (self, up) {
            (Self::English, true) => "Already on the first hand page",
            (Self::English, false) => "Already on the last hand page",
            (Self::Chinese, true) => "已经是手牌第一页",
            (Self::Chinese, false) => "已经是手牌最后一页",
        }
    }

    pub fn color_hint(self, mode: PlayMode, player_index: usize) -> &'static str {
        match (self, mode, player_index) {
            (Self::English, PlayMode::Dual, 0) => "A/D choose  Enter confirm  Esc cancel",
            (Self::Chinese, PlayMode::Dual, 0) => "A/D 选择  Enter 确认  Esc 取消",
            (Self::English, PlayMode::Dual, _) => "←/→ or h/l choose  Enter confirm  Esc cancel",
            (Self::Chinese, PlayMode::Dual, _) => "←/→ 或 h/l 选择  Enter 确认  Esc 取消",
            (_, PlayMode::Single, _) => self.text(Message::ColorHint),
        }
    }

    pub fn target_hint(self, mode: PlayMode, player_index: usize) -> &'static str {
        self.color_hint(mode, player_index)
    }

    pub fn enabled(self, enabled: bool) -> &'static str {
        self.text(if enabled {
            Message::Enabled
        } else {
            Message::Disabled
        })
    }

    pub fn swap_log(self, played: &str, target: &str) -> String {
        match self {
            Self::English => format!("{played} and swapped hands with {target}"),
            Self::Chinese => format!("{played}，并与 {target} 交换手牌"),
        }
    }

    pub fn rotate_log(self, played: &str, direction: Direction) -> String {
        match self {
            Self::English => format!("{played} and rotated hands {}", self.direction(direction)),
            Self::Chinese => format!("{played}，并按{}轮转手牌", self.direction(direction)),
        }
    }

    pub fn redistribute_log(self, played: &str, discarded: usize, distributed: usize) -> String {
        match self {
            Self::English => {
                format!("{played}, discarded {discarded} cards and distributed {distributed} cards")
            }
            Self::Chinese => {
                format!("{played}，弃置 {discarded} 张牌并向其他玩家均分 {distributed} 张牌")
            }
        }
    }

    pub fn hand_transform_log(
        self,
        played: &str,
        target: &str,
        before: usize,
        after: usize,
    ) -> String {
        match self {
            Self::English => {
                format!("{played}; {target}'s hand changed from {before} to {after} cards")
            }
            Self::Chinese => {
                format!("{played}；{target} 的手牌由 {before} 张变为 {after} 张")
            }
        }
    }

    pub fn plus_batch_log(
        self,
        player: &str,
        cards: usize,
        target: &str,
        penalty: usize,
        drawn: usize,
    ) -> String {
        match self {
            Self::English if drawn == penalty => {
                format!("{player} auto-played {cards} plus cards; {target} drew {penalty}")
            }
            Self::English => format!(
                "{player} auto-played {cards} plus cards; {target} drew {drawn} of {penalty}"
            ),
            Self::Chinese if drawn == penalty => {
                format!("{player} 自动打出 {cards} 张加牌；{target} 摸 {penalty} 张")
            }
            Self::Chinese => format!(
                "{player} 自动打出 {cards} 张加牌；{target} 应摸 {penalty} 张，实际摸 {drawn} 张"
            ),
        }
    }

    pub fn difficulty(self, difficulty: Difficulty) -> &'static str {
        self.text(match difficulty {
            Difficulty::Easy => Message::Easy,
            Difficulty::Normal => Message::Normal,
            Difficulty::Hard => Message::Hard,
            Difficulty::Extreme => Message::Extreme,
        })
    }

    pub fn deck_variant(self, variant: DeckVariant) -> &'static str {
        match (self, variant) {
            (Self::English, DeckVariant::Standard) => "Standard 112",
            (Self::English, DeckVariant::Holiday) => "Holiday 130",
            (Self::Chinese, DeckVariant::Standard) => "标准 112",
            (Self::Chinese, DeckVariant::Holiday) => "节日 130",
        }
    }

    /// 生成人类可读的图形设置摘要。
    ///
    /// `choice` 表示用户偏好，`backend` 表示环境探测后的实际结果。
    pub fn graphics(self, choice: GraphicsChoice, backend: GraphicsBackend) -> String {
        // 设置页同时展示用户选择与实际后端；Beta 模式降级时保留具体原因，
        // 便于用户判断是环境限制还是运行时编码错误。
        match (self, choice, backend) {
            (Self::English, GraphicsChoice::Text, _) => "Text".to_owned(),
            (Self::Chinese, GraphicsChoice::Text, _) => "文字".to_owned(),
            (Self::English, GraphicsChoice::GraphicsBeta, GraphicsBackend::Sixel) => {
                "Graphics (Beta) (Sixel)".to_owned()
            }
            (Self::English, GraphicsChoice::GraphicsBeta, GraphicsBackend::Termwiz) => {
                "Graphics (Beta) (Termwiz)".to_owned()
            }
            (Self::Chinese, GraphicsChoice::GraphicsBeta, GraphicsBackend::Sixel) => {
                "图像（Beta）（Sixel）".to_owned()
            }
            (Self::Chinese, GraphicsChoice::GraphicsBeta, GraphicsBackend::Termwiz) => {
                "图像（Beta）（Termwiz）".to_owned()
            }
            (Self::English, GraphicsChoice::GraphicsBeta, GraphicsBackend::Text(reason)) => {
                format!(
                    "Graphics (Beta) (Text: {})",
                    fallback_reason_english(reason)
                )
            }
            (Self::Chinese, GraphicsChoice::GraphicsBeta, GraphicsBackend::Text(reason)) => {
                format!("图像（Beta）（文字：{}）", fallback_reason_chinese(reason))
            }
        }
    }

    pub fn direction(self, direction: Direction) -> &'static str {
        self.text(match direction {
            Direction::Clockwise => Message::Clockwise,
            Direction::CounterClockwise => Message::CounterClockwise,
        })
    }

    pub fn color(self, color: Color) -> &'static str {
        match (self, color) {
            (Self::English, Color::Red) => "RED",
            (Self::English, Color::Yellow) => "YELLOW",
            (Self::English, Color::Green) => "GREEN",
            (Self::English, Color::Blue) => "BLUE",
            (Self::Chinese, Color::Red) => "红",
            (Self::Chinese, Color::Yellow) => "黄",
            (Self::Chinese, Color::Green) => "绿",
            (Self::Chinese, Color::Blue) => "蓝",
        }
    }

    pub fn card(self, card: Card) -> String {
        let color = card.color.map_or("WILD", |value| self.color(value));
        let rank = match (self, card.rank) {
            (_, Rank::Number(number)) => number.to_string(),
            (Self::English, Rank::Skip) => "SKIP".to_owned(),
            (Self::English, Rank::Reverse) => "REV".to_owned(),
            (Self::English, Rank::DrawTwo) => "+2".to_owned(),
            (Self::English, Rank::DrawEight) => "* +8 *".to_owned(),
            (Self::English, Rank::Wild) => "WILD".to_owned(),
            (Self::English, Rank::WildDrawFour) => "WILD +4".to_owned(),
            (Self::English, Rank::WildDrawSixteen) => "< WILD +16 >".to_owned(),
            (Self::English, Rank::WildDiscardThirtyTwo) => "< WILD -32 >".to_owned(),
            (Self::English, Rank::WildDiscardSixtyFour) => "< WILD -64 >".to_owned(),
            (Self::English, Rank::WildFactorial) => "< WILD x! >".to_owned(),
            (Self::English, Rank::WildSquareRoot) => "< WILD sqrt >".to_owned(),
            (Self::Chinese, Rank::Skip) => "禁".to_owned(),
            (Self::Chinese, Rank::Reverse) => "转".to_owned(),
            (Self::Chinese, Rank::DrawTwo) => "+2".to_owned(),
            (Self::Chinese, Rank::DrawEight) => "* +8 *".to_owned(),
            (Self::Chinese, Rank::Wild) => "变色".to_owned(),
            (Self::Chinese, Rank::WildDrawFour) => "变色 +4".to_owned(),
            (Self::Chinese, Rank::WildDrawSixteen) => "< 变色 +16 >".to_owned(),
            (Self::Chinese, Rank::WildDiscardThirtyTwo) => "< 变色 -32 >".to_owned(),
            (Self::Chinese, Rank::WildDiscardSixtyFour) => "< 变色 -64 >".to_owned(),
            (Self::Chinese, Rank::WildFactorial) => "< 变色 阶乘 >".to_owned(),
            (Self::Chinese, Rank::WildSquareRoot) => "< 变色 平方根 >".to_owned(),
        };
        if card.is_wild() {
            rank
        } else {
            format!("{color} {rank}")
        }
    }

    pub fn game_error(self, error: &GameError) -> String {
        let message = match error {
            GameError::NotPlayerTurn(_) => match self {
                Self::English => "It is not your turn",
                Self::Chinese => "还没轮到你",
            },
            GameError::CardNotOwned(_) => match self {
                Self::English => "You do not own that card",
                Self::Chinese => "你没有这张牌",
            },
            GameError::CardNotPlayable(_) => match self {
                Self::English => "That card cannot be played",
                Self::Chinese => "这张牌不能出",
            },
            GameError::DrawnCardOnly(_) => match self {
                Self::English => "Only the card just drawn can be played",
                Self::Chinese => "摸牌后只能打出刚摸到的牌",
            },
            GameError::MissingColorChoice => match self {
                Self::English => "Choose a color",
                Self::Chinese => "请选择颜色",
            },
            GameError::InvalidNumberBatchColor(_) => match self {
                Self::English => "Choose a color from the cards being played",
                Self::Chinese => "请选择本批出牌中包含的颜色",
            },
            GameError::MissingSwapTarget => match self {
                Self::English => "Choose a player to swap hands with",
                Self::Chinese => "请选择交换手牌的玩家",
            },
            GameError::UnexpectedSwapTarget => match self {
                Self::English => "That card does not accept a swap target",
                Self::Chinese => "这张牌不能指定换手目标",
            },
            GameError::InvalidSwapTarget(_) => match self {
                Self::English => "Choose another current player",
                Self::Chinese => "请选择另一名当前玩家",
            },
            GameError::WildDrawFourNotAllowed => match self {
                Self::English => "Wild +4 is illegal while you hold the active color",
                Self::Chinese => "手中有当前颜色时不能出变色 +4",
            },
            GameError::AlreadyDrew => match self {
                Self::English => "You already drew this turn",
                Self::Chinese => "本回合已经摸过牌",
            },
            GameError::CannotPassBeforeDrawing => match self {
                Self::English => "Draw before passing",
                Self::Chinese => "必须先摸牌才能跳过",
            },
            GameError::GameAlreadyWon => match self {
                Self::English => "The round is already over",
                Self::Chinese => "本局已经结束",
            },
            GameError::EmptyDrawPile => match self {
                Self::English => "No cards are available to draw; pass instead",
                Self::Chinese => "已经无牌可摸，请直接跳过",
            },
            GameError::EmptyPlusBatch | GameError::InvalidPlusBatch => match self {
                Self::English => "That +2/+8/+16 sequence cannot be played",
                Self::Chinese => "这组 +2/+8/+16 无法连续打出",
            },
            _ => match self {
                Self::English => "Action rejected",
                Self::Chinese => "操作被拒绝",
            },
        };
        message.to_owned()
    }
}

#[derive(Clone, Copy)]
pub enum Message {
    Title,
    Setup,
    Mode,
    PlayerOne,
    PlayerTwo,
    Bots,
    Difficulty,
    Deck,
    SevenZero,
    Enabled,
    Disabled,
    Language,
    Graphics,
    Start,
    SetupHint,
    Table,
    SelectedCard,
    DiscardTop,
    YourHand,
    NoMatchingCards,
    NoPlayablePlusBatch,
    EventLog,
    Turn,
    ActiveColor,
    Direction,
    Cards,
    ChooseColor,
    ChoosePlayer,
    ColorHint,
    Help,
    HelpBody,
    QuitTitle,
    QuitBody,
    Winner,
    NewMatchHint,
    TooSmall,
    Thinking,
    DrewCard,
    Passed,
    Played,
    InvalidCommand,
    InvalidCardIndex,
    Easy,
    Normal,
    Hard,
    Extreme,
    Clockwise,
    CounterClockwise,
}

fn fallback_reason_english(reason: FallbackReason) -> &'static str {
    match reason {
        FallbackReason::Manual => "manual",
        FallbackReason::Ssh => "SSH",
        FallbackReason::Tmux => "tmux",
        FallbackReason::Unsupported => "unsupported",
        FallbackReason::Encoding => "error",
    }
}

fn fallback_reason_chinese(reason: FallbackReason) -> &'static str {
    match reason {
        FallbackReason::Manual => "手动",
        FallbackReason::Ssh => "SSH",
        FallbackReason::Tmux => "tmux",
        FallbackReason::Unsupported => "不支持",
        FallbackReason::Encoding => "错误",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selects_chinese_and_falls_back_to_english() {
        assert_eq!(Language::from_locale(Some("zh-CN")), Language::Chinese);
        assert_eq!(Language::from_locale(Some("ZH_hans")), Language::Chinese);
        assert_eq!(Language::from_locale(Some("fr-FR")), Language::English);
        assert_eq!(Language::from_locale(None), Language::English);
        assert_eq!(Language::English.difficulty(Difficulty::Extreme), "Extreme");
        assert_eq!(Language::Chinese.difficulty(Difficulty::Extreme), "最难");
        assert_eq!(
            Language::English.deck_variant(DeckVariant::Standard),
            "Standard 112"
        );
        assert_eq!(
            Language::Chinese.deck_variant(DeckVariant::Holiday),
            "节日 130"
        );
        assert_eq!(
            Language::English.card(Card::wild(Rank::WildDiscardThirtyTwo)),
            "< WILD -32 >"
        );
        assert_eq!(
            Language::Chinese.card(Card::wild(Rank::WildDiscardSixtyFour)),
            "< 变色 -64 >"
        );
        assert_eq!(
            Language::English.card(Card::wild(Rank::WildFactorial)),
            "< WILD x! >"
        );
        assert_eq!(
            Language::Chinese.card(Card::wild(Rank::WildSquareRoot)),
            "< 变色 平方根 >"
        );
        assert_eq!(
            Language::Chinese.hand_transform_log("甲打出阶乘", "乙", 5, 120),
            "甲打出阶乘；乙 的手牌由 5 张变为 120 张"
        );
        assert_eq!(
            Language::English.redistribute_log("P played WILD -32", 32, 12),
            "P played WILD -32, discarded 32 cards and distributed 12 cards"
        );
        assert_eq!(
            Language::English.graphics(GraphicsChoice::GraphicsBeta, GraphicsBackend::Termwiz),
            "Graphics (Beta) (Termwiz)"
        );
        assert_eq!(
            Language::Chinese.graphics(
                GraphicsChoice::GraphicsBeta,
                GraphicsBackend::Text(FallbackReason::Ssh)
            ),
            "图像（Beta）（文字：SSH）"
        );
        assert_eq!(
            Language::English.graphics(
                GraphicsChoice::GraphicsBeta,
                GraphicsBackend::Text(FallbackReason::Unsupported)
            ),
            "Graphics (Beta) (Text: unsupported)"
        );
        assert_eq!(
            Language::English.graphics(
                GraphicsChoice::Text,
                GraphicsBackend::Text(FallbackReason::Manual)
            ),
            "Text"
        );
        assert_eq!(
            Language::Chinese.graphics(
                GraphicsChoice::Text,
                GraphicsBackend::Text(FallbackReason::Manual)
            ),
            "文字"
        );
    }

    #[test]
    fn navigation_hints_advertise_vim_keys_in_both_languages() {
        for language in Language::ALL {
            assert!(language.text(Message::SetupHint).contains("hjkl"));
            assert!(language.text(Message::HelpBody).contains("hjkl"));
            assert!(language.text(Message::ColorHint).contains("h/l"));
            assert!(language.text(Message::HelpBody).contains("hjkl"));
        }
    }
}
