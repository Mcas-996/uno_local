use std::env;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use uno_core::{Card, Color, Command, PlayerId, Rank};
use uno_net::{
    ConnectivityError, StunConfig, bind_udp, discover_public_endpoint, make_forwarded_share,
    send_join_request,
};
use uno_protocol::{JoinDecision, PeerId, RoomId, ShareInfo, StunStatus};
use uno_room::Room;

fn main() {
    if let Err(error) = run(env::args().collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.get(1).map(String::as_str) {
        Some("host") => host(&args[2..]),
        Some("join") => join(&args[2..]),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}'")),
    }
}

fn host(args: &[String]) -> Result<(), String> {
    let options = HostOptions::parse(args)?;
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], options.port));
    let socket = bind_udp(bind_addr).map_err(|error| error.to_string())?;
    let local_addr = socket.local_addr().map_err(|error| error.to_string())?;
    let room_id = RoomId::new(options.room.unwrap_or_else(|| short_id("room")));
    let peer_id = PeerId::new(options.peer.unwrap_or_else(|| short_id("host")));

    let endpoint = if let Some(forwarded) = options.forwarded {
        StunStatus::Manual(forwarded)
    } else if options.no_stun {
        StunStatus::Disabled
    } else {
        let stun_config = StunConfig {
            enabled: true,
            servers: options.stun_servers,
            timeout: Duration::from_millis(1500),
        };
        discover_public_endpoint(&socket, &stun_config)
            .unwrap_or_else(|error| StunStatus::Failed(error.to_string()))
    };

    let share = match endpoint {
        StunStatus::Discovered(endpoint) => {
            ShareInfo::new(endpoint, room_id.clone(), peer_id.clone())
        }
        StunStatus::Manual(endpoint) => {
            make_forwarded_share(endpoint, room_id.clone(), peer_id.clone())
        }
        StunStatus::Disabled | StunStatus::Failed(_) => {
            let fallback = SocketAddr::new(local_addr.ip(), local_addr.port());
            make_forwarded_share(fallback, room_id.clone(), peer_id.clone())
        }
    };

    println!("hosting room {}", room_id.0);
    println!("local udp: {local_addr}");
    println!("share: {}", share.encode());
    if matches!(endpoint, StunStatus::Failed(_) | StunStatus::Disabled) {
        print_port_forwarding_guidance(local_addr.port());
    }

    let mut room = Room::create(room_id, peer_id, options.name);
    interactive_session(&mut room, true, options.debug)
}

fn join(args: &[String]) -> Result<(), String> {
    let options = JoinOptions::parse(args)?;
    let share = ShareInfo::decode(&options.share).map_err(|error| error.to_string())?;
    let socket = bind_udp("0.0.0.0:0".parse().unwrap()).map_err(|error| error.to_string())?;
    println!("joining room {} at {}", share.room_id.0, share.endpoint);
    match send_join_request(
        &socket,
        &share,
        PeerId::new(options.peer.unwrap_or_else(|| short_id("peer"))),
        options.name,
        Duration::from_millis(1500),
    ) {
        Ok(JoinDecision::Accepted { session_id, role }) => {
            println!("joined: session={session_id}, role={role:?}");
            println!("connected. interactive remote play will use host-authoritative events.");
            Ok(())
        }
        Ok(JoinDecision::Rejected { reason }) => Err(format!("join rejected: {reason}")),
        Err(ConnectivityError::HandshakeTimeout) => {
            print_port_forwarding_guidance(share.endpoint.port());
            Err("handshake timed out".to_owned())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn interactive_session(
    room: &mut Room,
    local_active_host: bool,
    debug: bool,
) -> Result<(), String> {
    if debug {
        println!("debug mode: single-player start is enabled");
        println!(
            "commands: ready, start, state, hand, play <player> <color> <rank>, draw, pass, leave"
        );
    } else {
        println!("commands: ready, start, state, hand, play <color> <rank>, draw, pass, leave");
    }
    let mut line = String::new();
    loop {
        print!("uno> ");
        io::stdout().flush().map_err(|error| error.to_string())?;
        line.clear();
        if io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?
            == 0
        {
            println!("disconnect: stdin closed");
            return Ok(());
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "leave" {
            println!("left room");
            return Ok(());
        }
        if let Err(error) = handle_interactive(room, input, local_active_host, debug) {
            println!("{error}");
        }
    }
}

fn handle_interactive(
    room: &mut Room,
    input: &str,
    local_active_host: bool,
    debug: bool,
) -> Result<(), String> {
    let Some(host) = room.active_host().map(|member| member.peer_id.clone()) else {
        return Err("room has no active host".to_owned());
    };
    let host_player = PlayerId::new(host.0.clone());
    let parts: Vec<_> = input.split_whitespace().collect();
    match parts.first().copied() {
        Some("ready") => {
            room.set_ready(&host, true)
                .map_err(|error| error.to_string())?;
            println!("ready");
        }
        Some("start") => {
            if !local_active_host {
                return Err("only the active host can start the local debug room".to_owned());
            }
            if debug {
                room.start_debug_game(&host)
                    .map_err(|error| error.to_string())?;
            } else {
                room.start_game(&host).map_err(|error| error.to_string())?;
            }
            println!("game started");
        }
        Some("state") => println!("{:?}", room.public_game_state()),
        Some("hand") => println!("hand display is available after remote player state sync"),
        Some("draw") => {
            let event = room
                .submit_command(
                    &host,
                    Command::Draw {
                        player: host_player,
                    },
                )
                .map_err(|error| error.to_string())?;
            println!("event {:?}", event.kind);
        }
        Some("pass") => {
            let event = room
                .submit_command(
                    &host,
                    Command::Pass {
                        player: host_player,
                    },
                )
                .map_err(|error| error.to_string())?;
            println!("event {:?}", event.kind);
        }
        Some("play") => {
            if (debug && parts.len() < 4) || (!debug && parts.len() < 3) {
                return Err(play_usage(debug));
            }
            let (player, color_index, rank_index) = if debug {
                let player =
                    PlayerId::new(parts.get(1).ok_or_else(|| play_usage(debug))?.to_string());
                (player, 2, 3)
            } else {
                (host_player, 1, 2)
            };
            let color: Color = parts
                .get(color_index)
                .ok_or_else(|| play_usage(debug))?
                .parse()
                .map_err(|error| format!("{error}"))?;
            let rank = parse_rank(parts.get(rank_index).copied().unwrap_or(""))?;
            let card = if matches!(rank, Rank::Wild | Rank::WildDrawFour) {
                Card::wild(rank)
            } else {
                Card::new(color, rank)
            };
            let event = room
                .submit_command(
                    &host,
                    Command::Play {
                        player,
                        card,
                        chosen_color: card.is_wild().then_some(color),
                    },
                )
                .map_err(|error| error.to_string())?;
            println!("event {:?}", event.kind);
        }
        Some(other) => return Err(format!("invalid command '{other}'")),
        None => {}
    }
    Ok(())
}

fn play_usage(debug: bool) -> String {
    if debug {
        "usage: play <player> <color> <rank>".to_owned()
    } else {
        "usage: play <color> <rank>".to_owned()
    }
}

fn parse_rank(value: &str) -> Result<Rank, String> {
    match value {
        "skip" => Ok(Rank::Skip),
        "reverse" => Ok(Rank::Reverse),
        "draw-two" | "draw2" => Ok(Rank::DrawTwo),
        "wild" => Ok(Rank::Wild),
        "wild-draw-four" | "wild4" => Ok(Rank::WildDrawFour),
        number => number
            .parse::<u8>()
            .map(Rank::Number)
            .map_err(|_| format!("invalid rank '{value}'")),
    }
}

fn print_help() {
    println!(
        "uno host [--name NAME] [--port PORT] [--room ROOM] [--peer PEER] [--stun ADDR] [--no-stun] [--forwarded IP:PORT] [--debug]"
    );
    println!("uno join <share> [--name NAME] [--peer PEER]");
}

fn print_port_forwarding_guidance(port: u16) {
    println!("automatic public connectivity is unavailable.");
    println!(
        "fallback: forward UDP port {port} on the host router and share the forwarded endpoint."
    );
}

fn short_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        % 1_000_000;
    format!("{prefix}-{millis}")
}

#[derive(Debug)]
struct HostOptions {
    name: String,
    port: u16,
    room: Option<String>,
    peer: Option<String>,
    stun_servers: Vec<String>,
    no_stun: bool,
    forwarded: Option<SocketAddr>,
    debug: bool,
}

impl HostOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            name: "Host".to_owned(),
            port: 0,
            room: None,
            peer: None,
            stun_servers: StunConfig::default().servers,
            no_stun: false,
            forwarded: None,
            debug: false,
        };
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--name" => {
                    index += 1;
                    options.name = args.get(index).ok_or("--name requires a value")?.clone();
                }
                "--port" => {
                    index += 1;
                    options.port = args
                        .get(index)
                        .ok_or("--port requires a value")?
                        .parse()
                        .map_err(|_| "invalid port".to_owned())?;
                }
                "--room" => {
                    index += 1;
                    options.room = Some(args.get(index).ok_or("--room requires a value")?.clone());
                }
                "--peer" => {
                    index += 1;
                    options.peer = Some(args.get(index).ok_or("--peer requires a value")?.clone());
                }
                "--stun" => {
                    index += 1;
                    options
                        .stun_servers
                        .push(args.get(index).ok_or("--stun requires a value")?.clone());
                }
                "--no-stun" => options.no_stun = true,
                "--debug" => options.debug = true,
                "--forwarded" => {
                    index += 1;
                    options.forwarded = Some(
                        args.get(index)
                            .ok_or("--forwarded requires a value")?
                            .parse()
                            .map_err(|_| "invalid forwarded endpoint".to_owned())?,
                    );
                }
                other => return Err(format!("unknown host option '{other}'")),
            }
            index += 1;
        }
        Ok(options)
    }
}

#[derive(Debug)]
struct JoinOptions {
    share: String,
    name: String,
    peer: Option<String>,
}

impl JoinOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let share = args.first().ok_or("join requires a share string")?.clone();
        let mut options = Self {
            share,
            name: "Player".to_owned(),
            peer: None,
        };
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--name" => {
                    index += 1;
                    options.name = args.get(index).ok_or("--name requires a value")?.clone();
                }
                "--peer" => {
                    index += 1;
                    options.peer = Some(args.get(index).ok_or("--peer requires a value")?.clone());
                }
                other => return Err(format!("unknown join option '{other}'")),
            }
            index += 1;
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_options_parse_share_inputs() {
        let args = vec![
            "--name".to_owned(),
            "Alice".to_owned(),
            "--port".to_owned(),
            "34567".to_owned(),
            "--no-stun".to_owned(),
        ];
        let options = HostOptions::parse(&args).unwrap();
        assert_eq!(options.name, "Alice");
        assert_eq!(options.port, 34567);
        assert!(options.no_stun);
        assert!(!options.debug);
    }

    #[test]
    fn host_options_parse_debug() {
        let args = vec!["--debug".to_owned()];
        let options = HostOptions::parse(&args).unwrap();
        assert!(options.debug);
    }

    #[test]
    fn join_options_reject_missing_share() {
        assert!(JoinOptions::parse(&[]).is_err());
    }

    #[test]
    fn invalid_interactive_command_is_safe() {
        let mut room = Room::create(RoomId::new("room"), PeerId::new("host"), "Host".to_owned());
        assert!(
            handle_interactive(&mut room, "bad", true, false)
                .unwrap_err()
                .contains("invalid command")
        );
    }

    #[test]
    fn debug_start_accepts_single_host() {
        let mut room = Room::create(RoomId::new("room"), PeerId::new("host"), "Host".to_owned());
        handle_interactive(&mut room, "start", true, true).unwrap();
        assert!(room.public_game_state().is_some());
    }

    #[test]
    fn debug_play_requires_player() {
        let mut room = Room::create(RoomId::new("room"), PeerId::new("host"), "Host".to_owned());
        handle_interactive(&mut room, "start", true, true).unwrap();
        assert_eq!(
            handle_interactive(&mut room, "play red 5", true, true).unwrap_err(),
            "usage: play <player> <color> <rank>"
        );
    }
}
