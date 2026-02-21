use std::sync::Arc;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{info, debug, warn};

use crate::config::ServerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: SessionId,
    pub source_addr: SocketAddr,
    pub ssrc: u32,
    pub subscribers: DashMap<SocketAddr, Subscriber>,
    pub created_at: Instant,
    pub last_activity: RwLock<Instant>,
    pub packet_count: std::sync::atomic::AtomicU64,
    pub byte_count: std::sync::atomic::AtomicU64,
}

#[derive(Debug, Clone)]
pub struct Subscriber {
    pub addr: SocketAddr,
    pub joined_at: Instant,
    pub last_seq: u16,
    pub packet_count: std::sync::atomic::AtomicU64,
}

impl Session {
    pub fn new(id: SessionId, source_addr: SocketAddr, ssrc: u32) -> Self {
        let now = Instant::now();
        Self {
            id,
            source_addr,
            ssrc,
            subscribers: DashMap::new(),
            created_at: now,
            last_activity: RwLock::new(now),
            packet_count: std::sync::atomic::AtomicU64::new(0),
            byte_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn add_subscriber(&self, addr: SocketAddr) -> bool {
        let subscriber = Subscriber {
            addr,
            joined_at: Instant::now(),
            last_seq: 0,
            packet_count: std::sync::atomic::AtomicU64::new(0),
        };

        self.subscribers.insert(addr, subscriber);
        *self.last_activity.write() = Instant::now();
        
        info!("Added subscriber {} to session {} (total: {})", 
              addr, self.id.0, self.subscribers.len());
        true
    }

    pub fn remove_subscriber(&self, addr: &SocketAddr) -> bool {
        let removed = self.subscribers.remove(addr).is_some();
        if removed {
            *self.last_activity.write() = Instant::now();
            debug!("Removed subscriber {} from session {}", addr, self.id.0);
        }
        removed
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        let last = *self.last_activity.read();
        last.elapsed() > timeout
    }

    pub fn record_activity(&self) {
        *self.last_activity.write() = Instant::now();
    }
}

pub struct SessionManager {
    config: ServerConfig,
    sessions: DashMap<SessionId, Arc<Session>>,
    ssrc_index: DashMap<u32, SessionId>,
}

impl SessionManager {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            sessions: DashMap::with_capacity(1024),
            ssrc_index: DashMap::new(),
        }
    }

    pub fn create_session(&self, source_addr: SocketAddr, ssrc: u32) -> Option<Arc<Session>> {
        if self.sessions.len() >= self.config.max_sessions {
            warn!("Maximum session limit reached ({})", self.config.max_sessions);
            return None;
        }

        let id = SessionId::new();
        let session = Arc::new(Session::new(id, source_addr, ssrc));
        
        self.sessions.insert(id, session.clone());
        self.ssrc_index.insert(ssrc, id);
        
        info!("Created session {} for SSRC {} from {}", id.0, ssrc, source_addr);
        Some(session)
    }

    pub fn get_session(&self, id: &SessionId) -> Option<Arc<Session>> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub fn get_session_by_ssrc(&self, ssrc: u32) -> Option<Arc<Session>> {
        self.ssrc_index
            .get(&ssrc)
            .and_then(|id| self.get_session(&id))
    }

    pub fn remove_session(&self, id: &SessionId) -> bool {
        if let Some((_, session)) = self.sessions.remove(id) {
            self.ssrc_index.remove(&session.ssrc);
            info!("Removed session {}", id.0);
            true
        } else {
            false
        }
    }

    pub fn cleanup_expired_sessions(&self) {
        let timeout = Duration::from_secs(self.config.session_timeout_secs);
        let expired: Vec<_> = self
            .sessions
            .iter()
            .filter(|entry| entry.value().is_expired(timeout))
            .map(|entry| *entry.key())
            .collect();

        for id in expired {
            self.remove_session(&id);
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn total_subscribers(&self) -> usize {
        self.sessions
            .iter()
            .map(|s| s.subscribers.len())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new(
            SessionId::new(),
            "127.0.0.1:5004".parse().unwrap(),
            12345,
        );
        assert_eq!(session.ssrc, 12345);
    }

    #[test]
    fn test_session_add_subscriber() {
        let session = Session::new(
            SessionId::new(),
            "127.0.0.1:5004".parse().unwrap(),
            12345,
        );
        
        let addr: SocketAddr = "127.0.0.1:6000".parse().unwrap();
        assert!(session.add_subscriber(addr));
        assert_eq!(session.subscribers.len(), 1);
    }
}
