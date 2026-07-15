//! * STAR CARNIVAL WORDS *
//!
//! English and Chinese labels for every table and Holiday card.

use crate::ai::Difficulty;
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
            Message::PlayerName => ("Player name", "玩家名称"),
            Message::Bots => ("AI opponents", "电脑玩家"),
            Message::Difficulty => ("Difficulty", "难度"),
            Message::Deck => ("Deck", "牌组"),
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
            Message::EventLog => ("Events", "事件"),
            Message::Turn => ("Turn", "当前回合"),
            Message::ActiveColor => ("Active color", "当前颜色"),
            Message::Direction => ("Direction", "方向"),
            Message::Cards => ("cards", "张牌"),
            Message::GameUtilitiesHint => ("? help · Q quit", "? 帮助 · Q 退出"),
            Message::ChooseColor => ("Choose a color", "选择颜色"),
            Message::ColorHint => (
                "←/→ or h/l choose  Enter confirm  Esc cancel",
                "←/→ 或 h/l 选择  Enter 确认  Esc 取消",
            ),
            Message::Help => ("Help", "帮助"),
            Message::HelpBody => (
                "* STAR CARNIVAL *\n\nShortcuts\n  Arrows/hjkl select  Enter play\n  D draw              P pass\n  : command           Q quit\n\nHoliday\n  +8 matches color/rank\n  WILD +16 changes color\n\nCommands\n  play <index>  draw  pass\n  help          new   quit\n\nPress ? or Esc to return.",
                "* 星光嘉年华 *\n\n快捷键\n  方向键/hjkl 选择手牌  Enter 出牌\n  D 摸牌              P 跳过\n  : 输入命令          Q 退出\n\n节日牌\n  +8 匹配颜色或牌面\n  变色 +16 可改变颜色\n\n命令\n  play <序号>   draw  pass\n  help          new   quit\n\n按 ? 或 Esc 返回。",
            ),
            Message::QuitTitle => ("Leave match?", "退出对局？"),
            Message::QuitBody => ("Y confirm · N/Esc cancel", "Y 确认 · N/Esc 取消"),
            Message::Winner => ("Round complete", "本局结束"),
            Message::NewMatchHint => ("N new match · Q quit", "N 新游戏 · Q 退出"),
            Message::TooSmall => (
                "Terminal too small. Resize to at least 70 × 26.",
                "终端尺寸过小，请调整到至少 70 × 26。",
            ),
            Message::YourTurn => ("Your turn", "轮到你了"),
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

    pub fn default_player_name(self) -> &'static str {
        match self {
            Self::English => "Player",
            Self::Chinese => "玩家",
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
            (Self::English, DeckVariant::Standard) => "Standard 108",
            (Self::English, DeckVariant::Holiday) => "Holiday 118",
            (Self::Chinese, DeckVariant::Standard) => "标准 108",
            (Self::Chinese, DeckVariant::Holiday) => "节日 118",
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
            (Self::Chinese, Rank::Skip) => "禁".to_owned(),
            (Self::Chinese, Rank::Reverse) => "转".to_owned(),
            (Self::Chinese, Rank::DrawTwo) => "+2".to_owned(),
            (Self::Chinese, Rank::DrawEight) => "* +8 *".to_owned(),
            (Self::Chinese, Rank::Wild) => "变色".to_owned(),
            (Self::Chinese, Rank::WildDrawFour) => "变色 +4".to_owned(),
            (Self::Chinese, Rank::WildDrawSixteen) => "< 变色 +16 >".to_owned(),
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
    PlayerName,
    Bots,
    Difficulty,
    Deck,
    Language,
    Graphics,
    Start,
    SetupHint,
    Table,
    SelectedCard,
    DiscardTop,
    YourHand,
    EventLog,
    Turn,
    ActiveColor,
    Direction,
    Cards,
    GameUtilitiesHint,
    ChooseColor,
    ColorHint,
    Help,
    HelpBody,
    QuitTitle,
    QuitBody,
    Winner,
    NewMatchHint,
    TooSmall,
    YourTurn,
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
