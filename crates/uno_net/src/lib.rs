use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use uno_protocol::{
    HostRole, JoinDecision, Message, PeerId, RoomId, SessionId, ShareInfo, StunStatus,
    deserialize_message, serialize_message,
};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(1_500);
pub const DEFAULT_RETRY: Duration = Duration::from_millis(150);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StunConfig {
    pub enabled: bool,
    pub servers: Vec<String>,
    pub timeout: Duration,
}

impl Default for StunConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            servers: vec![
                "stun.l.google.com:19302".to_owned(),
                "stun1.l.google.com:19302".to_owned(),
            ],
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectivityError {
    Io(String),
    StunUnavailable(String),
    HandshakeTimeout,
    SessionTimeout,
    Protocol(String),
}

impl fmt::Display for ConnectivityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConnectivityError {}

impl From<io::Error> for ConnectivityError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub fn bind_udp(bind: SocketAddr) -> Result<UdpSocket, ConnectivityError> {
    let socket = UdpSocket::bind(bind)?;
    socket.set_read_timeout(Some(DEFAULT_TIMEOUT))?;
    socket.set_write_timeout(Some(DEFAULT_TIMEOUT))?;
    Ok(socket)
}

pub fn discover_public_endpoint(
    socket: &UdpSocket,
    config: &StunConfig,
) -> Result<StunStatus, ConnectivityError> {
    if !config.enabled {
        return Ok(StunStatus::Disabled);
    }

    let local = socket.local_addr()?;
    for server in &config.servers {
        let Some(server_addr) = resolve_first(server) else {
            continue;
        };
        if let Ok(endpoint) = stun_binding_request(socket, server_addr, config.timeout) {
            return Ok(StunStatus::Discovered(endpoint));
        }
    }

    if !local.ip().is_unspecified() {
        Ok(StunStatus::Manual(local))
    } else {
        Err(ConnectivityError::StunUnavailable(
            "no configured STUN server returned a public endpoint".to_owned(),
        ))
    }
}

pub fn make_forwarded_share(endpoint: SocketAddr, room_id: RoomId, host_id: PeerId) -> ShareInfo {
    ShareInfo::new(endpoint, room_id, host_id).with_forwarded(true)
}

pub fn host_receive_join(
    socket: &UdpSocket,
    room_id: &RoomId,
    timeout: Duration,
) -> Result<(Message, SocketAddr), ConnectivityError> {
    let start = Instant::now();
    let mut buf = [0_u8; 4096];
    while start.elapsed() < timeout {
        match socket.recv_from(&mut buf) {
            Ok((len, addr)) => {
                let text = std::str::from_utf8(&buf[..len])
                    .map_err(|error| ConnectivityError::Protocol(error.to_string()))?;
                let message = deserialize_message(text)
                    .map_err(|error| ConnectivityError::Protocol(error.to_string()))?;
                if let Message::JoinRequest {
                    room_id: requested, ..
                } = &message
                    && requested == room_id
                {
                    return Ok((message, addr));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(ConnectivityError::HandshakeTimeout)
}

pub fn send_join_request(
    socket: &UdpSocket,
    share: &ShareInfo,
    peer_id: PeerId,
    name: String,
    timeout: Duration,
) -> Result<JoinDecision, ConnectivityError> {
    let request = Message::JoinRequest {
        room_id: share.room_id.clone(),
        peer_id: peer_id.clone(),
        name,
    };
    let encoded = serialize_message(&request);
    let start = Instant::now();
    let mut buf = [0_u8; 4096];
    while start.elapsed() < timeout {
        socket.send_to(encoded.as_bytes(), share.endpoint)?;
        match socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let text = std::str::from_utf8(&buf[..len])
                    .map_err(|error| ConnectivityError::Protocol(error.to_string()))?;
                let message = deserialize_message(text)
                    .map_err(|error| ConnectivityError::Protocol(error.to_string()))?;
                if let Message::JoinResponse {
                    room_id,
                    peer_id: response_peer,
                    decision,
                } = message
                    && room_id == share.room_id
                    && response_peer == peer_id
                {
                    return Ok(decision);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::ConnectionReset
                ) =>
            {
                std::thread::sleep(DEFAULT_RETRY);
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(ConnectivityError::HandshakeTimeout)
}

pub fn send_join_response(
    socket: &UdpSocket,
    target: SocketAddr,
    room_id: RoomId,
    peer_id: PeerId,
    accepted: bool,
) -> Result<(), ConnectivityError> {
    let decision = if accepted {
        JoinDecision::Accepted {
            session_id: SessionId::new(format!("session-{peer_id}")),
            role: HostRole::Peer,
        }
    } else {
        JoinDecision::Rejected {
            reason: "room rejected join".to_owned(),
        }
    };
    let response = Message::JoinResponse {
        room_id,
        peer_id,
        decision,
    };
    socket.send_to(serialize_message(&response).as_bytes(), target)?;
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReliableMessage {
    pub session_id: SessionId,
    pub sequence: u64,
    pub payload: Message,
    pub sent_at: Option<Instant>,
    pub attempts: usize,
}

#[derive(Debug)]
pub struct ReliableQueue {
    retry_after: Duration,
    pending: VecDeque<ReliableMessage>,
    received: BTreeMap<SessionId, u64>,
}

impl ReliableQueue {
    pub fn new(retry_after: Duration) -> Self {
        Self {
            retry_after,
            pending: VecDeque::new(),
            received: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, session_id: SessionId, sequence: u64, payload: Message) {
        self.pending.push_back(ReliableMessage {
            session_id,
            sequence,
            payload,
            sent_at: None,
            attempts: 0,
        });
    }

    pub fn acknowledge(&mut self, session_id: &SessionId, sequence: u64) {
        self.pending
            .retain(|message| &message.session_id != session_id || message.sequence != sequence);
    }

    pub fn record_received(&mut self, session_id: SessionId, sequence: u64) -> bool {
        let last = self.received.entry(session_id).or_insert(0);
        if sequence <= *last {
            return false;
        }
        *last = sequence;
        true
    }

    pub fn due_messages(&mut self, now: Instant) -> Vec<Message> {
        let mut due = Vec::new();
        for message in &mut self.pending {
            let due_now = message
                .sent_at
                .map(|sent_at| now.duration_since(sent_at) >= self.retry_after)
                .unwrap_or(true);
            if due_now {
                message.sent_at = Some(now);
                message.attempts += 1;
                due.push(message.payload.clone());
            }
        }
        due
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

fn resolve_first(value: &str) -> Option<SocketAddr> {
    value.to_socket_addrs().ok()?.next()
}

fn stun_binding_request(
    socket: &UdpSocket,
    server: SocketAddr,
    timeout: Duration,
) -> Result<SocketAddr, ConnectivityError> {
    let transaction_id = *b"uno-client01";
    let mut request = Vec::with_capacity(20);
    request.extend_from_slice(&0x0001_u16.to_be_bytes());
    request.extend_from_slice(&0_u16.to_be_bytes());
    request.extend_from_slice(&0x2112_A442_u32.to_be_bytes());
    request.extend_from_slice(&transaction_id);

    socket.set_read_timeout(Some(timeout))?;
    socket.send_to(&request, server)?;

    let mut buf = [0_u8; 1024];
    let (len, _) = socket.recv_from(&mut buf)?;
    parse_stun_xor_mapped_address(&buf[..len])
        .ok_or_else(|| ConnectivityError::StunUnavailable("missing XOR-MAPPED-ADDRESS".to_owned()))
}

fn parse_stun_xor_mapped_address(bytes: &[u8]) -> Option<SocketAddr> {
    if bytes.len() < 20 {
        return None;
    }
    let magic = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if magic != 0x2112_A442 {
        return None;
    }
    let mut index = 20;
    while index + 4 <= bytes.len() {
        let attr_type = u16::from_be_bytes([bytes[index], bytes[index + 1]]);
        let attr_len = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]]) as usize;
        index += 4;
        if index + attr_len > bytes.len() {
            return None;
        }
        if attr_type == 0x0020 && attr_len >= 8 && bytes[index + 1] == 0x01 {
            let x_port = u16::from_be_bytes([bytes[index + 2], bytes[index + 3]])
                ^ ((0x2112_A442_u32 >> 16) as u16);
            let mut ip = [0_u8; 4];
            let magic_bytes = 0x2112_A442_u32.to_be_bytes();
            for offset in 0..4 {
                ip[offset] = bytes[index + 4 + offset] ^ magic_bytes[offset];
            }
            return Some(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from(ip),
                x_port,
            )));
        }
        index += attr_len.div_ceil(4) * 4;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn forwarded_share_marks_manual_endpoint() {
        let share = make_forwarded_share(
            "127.0.0.1:4444".parse().unwrap(),
            RoomId::new("room"),
            PeerId::new("host"),
        );
        assert!(share.forwarded);
        assert!(share.encode().contains("forwarded=1"));
    }

    #[test]
    fn local_udp_handshake_succeeds() {
        let host = bind_udp("127.0.0.1:0".parse().unwrap()).unwrap();
        let host_addr = host.local_addr().unwrap();
        let share = ShareInfo::new(host_addr, RoomId::new("room"), PeerId::new("host"));
        let host_thread = thread::spawn(move || {
            let (message, addr) =
                host_receive_join(&host, &RoomId::new("room"), Duration::from_secs(2)).unwrap();
            let Message::JoinRequest { peer_id, .. } = message else {
                panic!("expected join");
            };
            send_join_response(&host, addr, RoomId::new("room"), peer_id, true).unwrap();
        });

        let joiner = bind_udp("127.0.0.1:0".parse().unwrap()).unwrap();
        let decision = send_join_request(
            &joiner,
            &share,
            PeerId::new("joiner"),
            "Joiner".to_owned(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert!(matches!(decision, JoinDecision::Accepted { .. }));
        host_thread.join().unwrap();
    }

    #[test]
    fn handshake_timeout_is_reported() {
        let socket = bind_udp("127.0.0.1:0".parse().unwrap()).unwrap();
        let share = ShareInfo::new(
            "127.0.0.1:9".parse().unwrap(),
            RoomId::new("room"),
            PeerId::new("host"),
        );
        let error = send_join_request(
            &socket,
            &share,
            PeerId::new("joiner"),
            "Joiner".to_owned(),
            Duration::from_millis(100),
        )
        .unwrap_err();
        assert_eq!(error, ConnectivityError::HandshakeTimeout);
    }

    #[test]
    fn stun_disabled_surfaces_fallback_status() {
        let socket = bind_udp("127.0.0.1:0".parse().unwrap()).unwrap();
        let status = discover_public_endpoint(
            &socket,
            &StunConfig {
                enabled: false,
                servers: Vec::new(),
                timeout: Duration::from_millis(10),
            },
        )
        .unwrap();
        assert_eq!(status, StunStatus::Disabled);
    }

    #[test]
    fn reliable_queue_retries_and_deduplicates() {
        let mut queue = ReliableQueue::new(Duration::from_millis(10));
        let session = SessionId::new("s");
        queue.push(
            session.clone(),
            1,
            Message::Ping {
                session_id: session.clone(),
                sequence: 1,
            },
        );
        assert_eq!(queue.due_messages(Instant::now()).len(), 1);
        assert_eq!(queue.pending_len(), 1);
        queue.acknowledge(&session, 1);
        assert_eq!(queue.pending_len(), 0);
        assert!(queue.record_received(session.clone(), 2));
        assert!(!queue.record_received(session, 2));
    }
}
