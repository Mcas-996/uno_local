use std::collections::BTreeMap;
use std::fmt;

use uno_core::{Command, Game, GameError, GameEvent, PlayerId, PublicGameState};
use uno_protocol::{HostRole, MemberInfo, PeerId, RoomId, SessionId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Member {
    pub peer_id: PeerId,
    pub player_id: PlayerId,
    pub name: String,
    pub ready: bool,
    pub connected: bool,
    pub role: HostRole,
    pub synchronized_sequence: u64,
}

impl Member {
    pub fn info(&self) -> MemberInfo {
        MemberInfo {
            peer_id: self.peer_id.clone(),
            player_id: self.player_id.clone(),
            name: self.name.clone(),
            ready: self.ready,
            connected: self.connected,
            role: self.role,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplicatedState {
    pub members: Vec<Member>,
    pub game: Option<Game>,
    pub event_sequence: u64,
    pub pending_critical_sequences: Vec<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoomStatus {
    Lobby,
    InGame,
    Suspended(String),
    Ended(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Room {
    pub id: RoomId,
    pub session_id: SessionId,
    members: BTreeMap<PeerId, Member>,
    status: RoomStatus,
    game: Option<Game>,
    pending_critical_sequences: Vec<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RoomError {
    DuplicatePeer(PeerId),
    UnknownPeer(PeerId),
    NotActiveHost(PeerId),
    InvalidPlayerCount(usize),
    GameAlreadyStarted,
    GameNotStarted,
    NoSynchronizedStandby,
    Gameplay(String),
}

impl fmt::Display for RoomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RoomError {}

impl From<GameError> for RoomError {
    fn from(value: GameError) -> Self {
        RoomError::Gameplay(value.to_string())
    }
}

impl Room {
    pub fn create(id: RoomId, host_peer: PeerId, host_name: String) -> Self {
        let session_id = SessionId::new(format!("session-{id}"));
        let player_id = PlayerId::new(host_peer.0.clone());
        let host = Member {
            peer_id: host_peer.clone(),
            player_id,
            name: host_name,
            ready: false,
            connected: true,
            role: HostRole::Active,
            synchronized_sequence: 0,
        };
        let mut members = BTreeMap::new();
        members.insert(host_peer, host);
        Self {
            id,
            session_id,
            members,
            status: RoomStatus::Lobby,
            game: None,
            pending_critical_sequences: Vec::new(),
        }
    }

    pub fn join(&mut self, peer_id: PeerId, name: String) -> Result<(), RoomError> {
        if self.members.contains_key(&peer_id) {
            return Err(RoomError::DuplicatePeer(peer_id));
        }
        let member = Member {
            player_id: PlayerId::new(peer_id.0.clone()),
            peer_id: peer_id.clone(),
            name,
            ready: false,
            connected: true,
            role: HostRole::Peer,
            synchronized_sequence: self.event_sequence(),
        };
        self.members.insert(peer_id, member);
        self.assign_standby();
        Ok(())
    }

    pub fn leave(&mut self, peer_id: &PeerId) -> Result<(), RoomError> {
        let member = self
            .members
            .get_mut(peer_id)
            .ok_or_else(|| RoomError::UnknownPeer(peer_id.clone()))?;
        member.connected = false;
        member.ready = false;
        if member.role == HostRole::Active {
            self.promote_standby()?;
        } else if member.role == HostRole::Standby {
            member.role = HostRole::Peer;
            self.assign_standby();
        }
        Ok(())
    }

    pub fn set_ready(&mut self, peer_id: &PeerId, ready: bool) -> Result<(), RoomError> {
        let member = self
            .members
            .get_mut(peer_id)
            .ok_or_else(|| RoomError::UnknownPeer(peer_id.clone()))?;
        member.ready = ready;
        Ok(())
    }

    pub fn start_game(&mut self, requester: &PeerId) -> Result<(), RoomError> {
        self.ensure_active_host(requester)?;
        if self.game.is_some() {
            return Err(RoomError::GameAlreadyStarted);
        }
        let players = self
            .connected_members()
            .into_iter()
            .map(|member| (member.player_id.clone(), member.name.clone()))
            .collect::<Vec<_>>();
        if !(uno_core::MIN_PLAYERS..=uno_core::MAX_PLAYERS_V1).contains(&players.len()) {
            return Err(RoomError::InvalidPlayerCount(players.len()));
        }
        self.game = Some(Game::new(players)?);
        self.status = RoomStatus::InGame;
        self.replicate_to_standby();
        Ok(())
    }

    pub fn submit_command(
        &mut self,
        requester: &PeerId,
        command: Command,
    ) -> Result<GameEvent, RoomError> {
        self.ensure_active_host(requester)?;
        let game = self.game.as_mut().ok_or(RoomError::GameNotStarted)?;
        let event = game.apply_command(command)?;
        self.pending_critical_sequences.push(event.sequence);
        self.replicate_to_standby();
        Ok(event)
    }

    pub fn public_game_state(&self) -> Option<PublicGameState> {
        self.game.as_ref().map(Game::public_state)
    }

    pub fn members(&self) -> Vec<Member> {
        self.members.values().cloned().collect()
    }

    pub fn member_infos(&self) -> Vec<MemberInfo> {
        self.members.values().map(Member::info).collect()
    }

    pub fn status(&self) -> &RoomStatus {
        &self.status
    }

    pub fn active_host(&self) -> Option<&Member> {
        self.members
            .values()
            .find(|member| member.connected && member.role == HostRole::Active)
    }

    pub fn standby_host(&self) -> Option<&Member> {
        self.members
            .values()
            .find(|member| member.connected && member.role == HostRole::Standby)
    }

    pub fn replicated_state(&self) -> ReplicatedState {
        ReplicatedState {
            members: self.members(),
            game: self.game.clone(),
            event_sequence: self.event_sequence(),
            pending_critical_sequences: self.pending_critical_sequences.clone(),
        }
    }

    pub fn apply_replicated_state(&mut self, state: ReplicatedState) {
        self.members = state
            .members
            .into_iter()
            .map(|member| (member.peer_id.clone(), member))
            .collect();
        self.game = state.game;
        self.pending_critical_sequences = state.pending_critical_sequences;
    }

    pub fn promote_standby(&mut self) -> Result<(), RoomError> {
        let current_sequence = self.event_sequence();
        let standby_peer = self
            .members
            .values()
            .find(|member| {
                member.connected
                    && member.role == HostRole::Standby
                    && member.synchronized_sequence >= current_sequence
            })
            .map(|member| member.peer_id.clone());

        let Some(standby_peer) = standby_peer else {
            self.status = RoomStatus::Suspended("no synchronized standby host".to_owned());
            return Err(RoomError::NoSynchronizedStandby);
        };

        for member in self.members.values_mut() {
            if member.role == HostRole::Active {
                member.role = HostRole::Peer;
                member.connected = false;
            }
            if member.peer_id == standby_peer {
                member.role = HostRole::Active;
            }
        }
        self.assign_standby();
        Ok(())
    }

    pub fn end_if_no_host(&mut self) {
        if self.active_host().is_none() {
            self.status = RoomStatus::Ended("no active host can continue".to_owned());
        }
    }

    fn ensure_active_host(&self, requester: &PeerId) -> Result<(), RoomError> {
        match self.active_host() {
            Some(member) if &member.peer_id == requester => Ok(()),
            _ => Err(RoomError::NotActiveHost(requester.clone())),
        }
    }

    fn connected_members(&self) -> Vec<Member> {
        self.members
            .values()
            .filter(|member| member.connected)
            .cloned()
            .collect()
    }

    fn assign_standby(&mut self) {
        if self.standby_host().is_some() {
            return;
        }
        let active = self.active_host().map(|member| member.peer_id.clone());
        let sequence = self.event_sequence();
        for member in self.members.values_mut() {
            if Some(&member.peer_id) != active.as_ref() && member.connected {
                member.role = HostRole::Standby;
                member.synchronized_sequence = sequence;
                break;
            }
        }
    }

    fn replicate_to_standby(&mut self) {
        let sequence = self.event_sequence();
        for member in self.members.values_mut() {
            if member.role == HostRole::Standby && member.connected {
                member.synchronized_sequence = sequence;
            }
        }
    }

    fn event_sequence(&self) -> u64 {
        self.game.as_ref().map(Game::next_sequence).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room() -> Room {
        Room::create(RoomId::new("room"), PeerId::new("host"), "Host".to_owned())
    }

    #[test]
    fn join_leave_and_ready_work() {
        let mut room = room();
        room.join(PeerId::new("peer"), "Peer".to_owned()).unwrap();
        room.set_ready(&PeerId::new("peer"), true).unwrap();
        assert_eq!(room.members().len(), 2);
        assert!(room.members().iter().any(|member| member.ready));
        room.leave(&PeerId::new("peer")).unwrap();
        assert!(
            !room
                .members()
                .iter()
                .find(|member| member.peer_id == PeerId::new("peer"))
                .unwrap()
                .connected
        );
    }

    #[test]
    fn standby_is_assigned() {
        let mut room = room();
        room.join(PeerId::new("peer"), "Peer".to_owned()).unwrap();
        assert_eq!(room.active_host().unwrap().peer_id, PeerId::new("host"));
        assert_eq!(room.standby_host().unwrap().peer_id, PeerId::new("peer"));
    }

    #[test]
    fn active_host_disconnect_promotes_standby() {
        let mut room = room();
        room.join(PeerId::new("peer"), "Peer".to_owned()).unwrap();
        room.leave(&PeerId::new("host")).unwrap();
        assert_eq!(room.active_host().unwrap().peer_id, PeerId::new("peer"));
    }

    #[test]
    fn no_host_continuation_fails() {
        let mut room = room();
        let result = room.leave(&PeerId::new("host"));
        assert_eq!(result.unwrap_err(), RoomError::NoSynchronizedStandby);
        assert!(matches!(room.status(), RoomStatus::Suspended(_)));
    }

    #[test]
    fn start_game_rejects_invalid_player_count() {
        let mut room = room();
        assert_eq!(
            room.start_game(&PeerId::new("host")).unwrap_err(),
            RoomError::InvalidPlayerCount(1)
        );
    }

    #[test]
    fn host_and_joiner_complete_minimal_game_flow() {
        let mut room = room();
        room.join(PeerId::new("peer"), "Peer".to_owned()).unwrap();
        room.start_game(&PeerId::new("host")).unwrap();
        assert!(matches!(room.status(), RoomStatus::InGame));
        assert!(room.public_game_state().is_some());
    }

    #[test]
    fn replicated_state_can_seed_standby_room() {
        let mut room = room();
        room.join(PeerId::new("peer"), "Peer".to_owned()).unwrap();
        room.start_game(&PeerId::new("host")).unwrap();
        let replicated = room.replicated_state();

        let mut standby = Room::create(RoomId::new("room"), PeerId::new("peer"), "Peer".to_owned());
        standby.apply_replicated_state(replicated);
        standby.leave(&PeerId::new("host")).unwrap();
        assert_eq!(standby.active_host().unwrap().peer_id, PeerId::new("peer"));
    }
}
