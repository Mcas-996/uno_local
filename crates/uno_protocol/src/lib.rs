use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use uno_core::{Card, Command, GameEvent, PlayerId, Rank};

pub const PROTOCOL_VERSION: &str = "v1";
pub const DEFAULT_TRANSPORT: &str = "udp";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PeerId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RoomId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SessionId(pub String);

impl PeerId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl RoomId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl SessionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostRole {
    Active,
    Standby,
    Peer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShareInfo {
    pub version: String,
    pub transport: String,
    pub endpoint: SocketAddr,
    pub room_id: RoomId,
    pub host_id: PeerId,
    pub forwarded: bool,
}

impl ShareInfo {
    pub fn new(endpoint: SocketAddr, room_id: RoomId, host_id: PeerId) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_owned(),
            transport: DEFAULT_TRANSPORT.to_owned(),
            endpoint,
            room_id,
            host_id,
            forwarded: false,
        }
    }

    pub fn with_forwarded(mut self, forwarded: bool) -> Self {
        self.forwarded = forwarded;
        self
    }

    pub fn encode(&self) -> String {
        format!(
            "uno://{}/{}/{}/room/{}/host/{}{}",
            self.version,
            self.transport,
            self.endpoint,
            escape(&self.room_id.0),
            escape(&self.host_id.0),
            if self.forwarded { "?forwarded=1" } else { "" }
        )
    }

    pub fn decode(value: &str) -> Result<Self, ProtocolError> {
        let rest = value
            .strip_prefix("uno://")
            .ok_or_else(|| ProtocolError::InvalidShare("missing uno:// prefix".to_owned()))?;
        let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != 7 || parts[3] != "room" || parts[5] != "host" {
            return Err(ProtocolError::InvalidShare(value.to_owned()));
        }
        if parts[0] != PROTOCOL_VERSION || parts[1] != DEFAULT_TRANSPORT {
            return Err(ProtocolError::InvalidShare(value.to_owned()));
        }
        let endpoint = parts[2]
            .parse()
            .map_err(|_| ProtocolError::InvalidEndpoint(parts[2].to_owned()))?;
        Ok(Self {
            version: parts[0].to_owned(),
            transport: parts[1].to_owned(),
            endpoint,
            room_id: RoomId(unescape(parts[4])),
            host_id: PeerId(unescape(parts[6])),
            forwarded: query.split('&').any(|part| part == "forwarded=1"),
        })
    }
}

impl fmt::Display for ShareInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

impl FromStr for ShareInfo {
    type Err = ProtocolError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::decode(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StunStatus {
    Disabled,
    Failed(String),
    Discovered(SocketAddr),
    Manual(SocketAddr),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JoinDecision {
    Accepted {
        session_id: SessionId,
        role: HostRole,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisconnectReason {
    Left,
    Timeout,
    HostUnavailable,
    ProtocolError(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Message {
    Ping {
        session_id: SessionId,
        sequence: u64,
    },
    Ack {
        session_id: SessionId,
        sequence: u64,
    },
    JoinRequest {
        room_id: RoomId,
        peer_id: PeerId,
        name: String,
    },
    JoinResponse {
        room_id: RoomId,
        peer_id: PeerId,
        decision: JoinDecision,
    },
    Membership {
        room_id: RoomId,
        members: Vec<MemberInfo>,
    },
    Ready {
        room_id: RoomId,
        peer_id: PeerId,
        ready: bool,
    },
    Command {
        room_id: RoomId,
        sequence: u64,
        command: Command,
    },
    Event {
        room_id: RoomId,
        event: GameEvent,
    },
    Disconnect {
        room_id: RoomId,
        peer_id: PeerId,
        reason: DisconnectReason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberInfo {
    pub peer_id: PeerId,
    pub player_id: PlayerId,
    pub name: String,
    pub ready: bool,
    pub connected: bool,
    pub role: HostRole,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidShare(String),
    InvalidEndpoint(String),
    InvalidMessage(String),
    UnknownMessageType(String),
    MissingField(&'static str),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ProtocolError {}

pub fn serialize_message(message: &Message) -> String {
    match message {
        Message::Ping {
            session_id,
            sequence,
        } => format!("PING|{session_id}|{sequence}"),
        Message::Ack {
            session_id,
            sequence,
        } => format!("ACK|{session_id}|{sequence}"),
        Message::JoinRequest {
            room_id,
            peer_id,
            name,
        } => format!("JOIN|{room_id}|{peer_id}|{}", escape(name)),
        Message::JoinResponse {
            room_id,
            peer_id,
            decision,
        } => match decision {
            JoinDecision::Accepted { session_id, role } => format!(
                "JOIN_OK|{room_id}|{peer_id}|{session_id}|{}",
                encode_role(*role)
            ),
            JoinDecision::Rejected { reason } => {
                format!("JOIN_NO|{room_id}|{peer_id}|{}", escape(reason))
            }
        },
        Message::Membership { room_id, members } => {
            let members = members
                .iter()
                .map(serialize_member)
                .collect::<Vec<_>>()
                .join(",");
            format!("MEMBERS|{room_id}|{members}")
        }
        Message::Ready {
            room_id,
            peer_id,
            ready,
        } => format!("READY|{room_id}|{peer_id}|{}", u8::from(*ready)),
        Message::Command {
            room_id,
            sequence,
            command,
        } => format!(
            "COMMAND|{room_id}|{sequence}|{}",
            serialize_command(command)
        ),
        Message::Event { room_id, event } => {
            format!(
                "EVENT|{room_id}|{}|{}",
                event.sequence,
                serialize_event(event)
            )
        }
        Message::Disconnect {
            room_id,
            peer_id,
            reason,
        } => format!(
            "DISCONNECT|{room_id}|{peer_id}|{}",
            escape(&format!("{reason:?}"))
        ),
    }
}

pub fn deserialize_message(value: &str) -> Result<Message, ProtocolError> {
    let parts: Vec<_> = value.split('|').collect();
    let kind = parts
        .first()
        .ok_or_else(|| ProtocolError::InvalidMessage(value.to_owned()))?;
    match *kind {
        "PING" => Ok(Message::Ping {
            session_id: SessionId(req(&parts, 1, "session_id")?.to_owned()),
            sequence: parse_u64(req(&parts, 2, "sequence")?)?,
        }),
        "ACK" => Ok(Message::Ack {
            session_id: SessionId(req(&parts, 1, "session_id")?.to_owned()),
            sequence: parse_u64(req(&parts, 2, "sequence")?)?,
        }),
        "JOIN" => Ok(Message::JoinRequest {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            peer_id: PeerId(req(&parts, 2, "peer_id")?.to_owned()),
            name: unescape(req(&parts, 3, "name")?),
        }),
        "JOIN_OK" => Ok(Message::JoinResponse {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            peer_id: PeerId(req(&parts, 2, "peer_id")?.to_owned()),
            decision: JoinDecision::Accepted {
                session_id: SessionId(req(&parts, 3, "session_id")?.to_owned()),
                role: decode_role(req(&parts, 4, "role")?)?,
            },
        }),
        "JOIN_NO" => Ok(Message::JoinResponse {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            peer_id: PeerId(req(&parts, 2, "peer_id")?.to_owned()),
            decision: JoinDecision::Rejected {
                reason: unescape(req(&parts, 3, "reason")?),
            },
        }),
        "MEMBERS" => Ok(Message::Membership {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            members: parse_members(req(&parts, 2, "members")?)?,
        }),
        "READY" => Ok(Message::Ready {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            peer_id: PeerId(req(&parts, 2, "peer_id")?.to_owned()),
            ready: req(&parts, 3, "ready")? == "1",
        }),
        "COMMAND" => Ok(Message::Command {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            sequence: parse_u64(req(&parts, 2, "sequence")?)?,
            command: parse_command(req(&parts, 3, "command")?)?,
        }),
        "EVENT" => Ok(Message::Event {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            event: GameEvent {
                sequence: parse_u64(req(&parts, 2, "sequence")?)?,
                kind: uno_core::EventKind::GameStarted,
            },
        }),
        "DISCONNECT" => Ok(Message::Disconnect {
            room_id: RoomId(req(&parts, 1, "room_id")?.to_owned()),
            peer_id: PeerId(req(&parts, 2, "peer_id")?.to_owned()),
            reason: DisconnectReason::ProtocolError(unescape(req(&parts, 3, "reason")?)),
        }),
        other => Err(ProtocolError::UnknownMessageType(other.to_owned())),
    }
}

pub fn serialize_command(command: &Command) -> String {
    match command {
        Command::Play {
            player,
            card,
            chosen_color,
        } => format!(
            "play:{}:{}:{}",
            escape(&player.0),
            encode_card(*card),
            chosen_color
                .map(|color| color.to_string())
                .unwrap_or_else(|| "-".to_owned())
        ),
        Command::Draw { player } => format!("draw:{}", escape(&player.0)),
        Command::Pass { player } => format!("pass:{}", escape(&player.0)),
        Command::ChooseColor { player, color } => {
            format!("color:{}:{color}", escape(&player.0))
        }
    }
}

pub fn parse_command(value: &str) -> Result<Command, ProtocolError> {
    let parts: Vec<_> = value.split(':').collect();
    match *parts
        .first()
        .ok_or_else(|| ProtocolError::InvalidMessage(value.to_owned()))?
    {
        "play" => {
            let chosen = req(&parts, 3, "chosen_color")?;
            Ok(Command::Play {
                player: PlayerId(unescape(req(&parts, 1, "player")?)),
                card: decode_card(req(&parts, 2, "card")?)?,
                chosen_color: if chosen == "-" {
                    None
                } else {
                    Some(chosen.parse().map_err(|_| {
                        ProtocolError::InvalidMessage(format!("invalid color {chosen}"))
                    })?)
                },
            })
        }
        "draw" => Ok(Command::Draw {
            player: PlayerId(unescape(req(&parts, 1, "player")?)),
        }),
        "pass" => Ok(Command::Pass {
            player: PlayerId(unescape(req(&parts, 1, "player")?)),
        }),
        "color" => Ok(Command::ChooseColor {
            player: PlayerId(unescape(req(&parts, 1, "player")?)),
            color: req(&parts, 2, "color")?
                .parse()
                .map_err(|_| ProtocolError::InvalidMessage(value.to_owned()))?,
        }),
        _ => Err(ProtocolError::InvalidMessage(value.to_owned())),
    }
}

fn serialize_event(event: &GameEvent) -> String {
    format!("{:?}", event.kind)
}

fn serialize_member(member: &MemberInfo) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        escape(&member.peer_id.0),
        escape(&member.player_id.0),
        escape(&member.name),
        u8::from(member.ready),
        u8::from(member.connected),
        encode_role(member.role)
    )
}

fn parse_members(value: &str) -> Result<Vec<MemberInfo>, ProtocolError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split(',')
        .map(|member| {
            let parts: Vec<_> = member.split(':').collect();
            Ok(MemberInfo {
                peer_id: PeerId(unescape(req(&parts, 0, "peer_id")?)),
                player_id: PlayerId(unescape(req(&parts, 1, "player_id")?)),
                name: unescape(req(&parts, 2, "name")?),
                ready: req(&parts, 3, "ready")? == "1",
                connected: req(&parts, 4, "connected")? == "1",
                role: decode_role(req(&parts, 5, "role")?)?,
            })
        })
        .collect()
}

fn encode_role(role: HostRole) -> &'static str {
    match role {
        HostRole::Active => "active",
        HostRole::Standby => "standby",
        HostRole::Peer => "peer",
    }
}

fn decode_role(value: &str) -> Result<HostRole, ProtocolError> {
    match value {
        "active" => Ok(HostRole::Active),
        "standby" => Ok(HostRole::Standby),
        "peer" => Ok(HostRole::Peer),
        _ => Err(ProtocolError::InvalidMessage(format!(
            "invalid role {value}"
        ))),
    }
}

fn encode_card(card: Card) -> String {
    match card.color {
        Some(color) => format!("{}-{}", color, card.rank),
        None => format!("wild-{}", card.rank),
    }
}

fn decode_card(value: &str) -> Result<Card, ProtocolError> {
    let (color, rank) = value
        .split_once('-')
        .ok_or_else(|| ProtocolError::InvalidMessage(format!("invalid card {value}")))?;
    let rank = match rank {
        "skip" => Rank::Skip,
        "reverse" => Rank::Reverse,
        "draw-two" => Rank::DrawTwo,
        "wild" => Rank::Wild,
        "wild-draw-four" => Rank::WildDrawFour,
        number => Rank::Number(
            number
                .parse()
                .map_err(|_| ProtocolError::InvalidMessage(format!("invalid rank {rank}")))?,
        ),
    };
    if color == "wild" {
        Ok(Card::wild(rank))
    } else {
        Ok(Card::new(
            color
                .parse()
                .map_err(|_| ProtocolError::InvalidMessage(format!("invalid color {color}")))?,
            rank,
        ))
    }
}

fn req<'a>(parts: &'a [&str], index: usize, field: &'static str) -> Result<&'a str, ProtocolError> {
    parts
        .get(index)
        .copied()
        .ok_or(ProtocolError::MissingField(field))
}

fn parse_u64(value: &str) -> Result<u64, ProtocolError> {
    value
        .parse()
        .map_err(|_| ProtocolError::InvalidMessage(value.to_owned()))
}

fn escape(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('|', "%7C")
        .replace('/', "%2F")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

fn unescape(value: &str) -> String {
    value
        .replace("%2C", ",")
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%7C", "|")
        .replace("%25", "%")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uno_core::{Color, Rank};

    #[test]
    fn share_string_round_trips() {
        let share = ShareInfo::new(
            "127.0.0.1:34567".parse().unwrap(),
            RoomId::new("room-a"),
            PeerId::new("host-a"),
        )
        .with_forwarded(true);
        let parsed = ShareInfo::decode(&share.encode()).unwrap();
        assert_eq!(parsed, share);
    }

    #[test]
    fn invalid_share_string_is_rejected() {
        assert!(ShareInfo::decode("bad").is_err());
    }

    #[test]
    fn command_message_round_trips() {
        let message = Message::Command {
            room_id: RoomId::new("r1"),
            sequence: 7,
            command: Command::Play {
                player: PlayerId::new("p1"),
                card: Card::new(Color::Red, Rank::Number(5)),
                chosen_color: None,
            },
        };
        assert_eq!(
            deserialize_message(&serialize_message(&message)).unwrap(),
            message
        );
    }
}
