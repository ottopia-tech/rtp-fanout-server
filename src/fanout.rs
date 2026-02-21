use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use crossbeam::queue::SegQueue;
use tracing::{debug, trace, warn};
use dashmap::DashMap;

use crate::session::{SessionManager, Session};
use crate::RtpPacket;

pub struct FanoutEngine {
    session_manager: Arc<SessionManager>,
    packet_queue: Arc<SegQueue<RtpPacket>>,
    socket: DashMap<SocketAddr, Arc<UdpSocket>>,
}

impl FanoutEngine {
    pub fn new(
        session_manager: Arc<SessionManager>,
        packet_queue: Arc<SegQueue<RtpPacket>>,
    ) -> Self {
        Self {
            session_manager,
            packet_queue,
            socket: DashMap::new(),
        }
    }

    pub async fn process_batch(&self) {
        const BATCH_SIZE: usize = 256;
        
        for _ in 0..BATCH_SIZE {
            if let Some(packet) = self.packet_queue.pop() {
                self.fanout_packet(&packet).await;
            } else {
                break;
            }
        }
    }

    async fn fanout_packet(&self, packet: &RtpPacket) {
        if let Some(session) = self.session_manager.get_session_by_ssrc(packet.ssrc) {
            session.record_activity();
            
            session.packet_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            session.byte_count.fetch_add(
                packet.payload.len() as u64, 
                std::sync::atomic::Ordering::Relaxed
            );

            let rtp_data = self.serialize_rtp_packet(packet);
            
            let subscribers: Vec<_> = session
                .subscribers
                .iter()
                .map(|entry| *entry.key())
                .collect();

            for subscriber_addr in subscribers {
                self.send_to_subscriber(&rtp_data, subscriber_addr).await;
            }

            trace!("Fanned out packet seq={} to {} subscribers", 
                   packet.sequence, session.subscribers.len());
        } else {
            debug!("No session found for SSRC {}", packet.ssrc);
        }
    }

    fn serialize_rtp_packet(&self, packet: &RtpPacket) -> Vec<u8> {
        let mut data = Vec::with_capacity(12 + packet.payload.len());
        
        data.push(0x80);
        
        let pt_byte = if packet.marker { 0x80 } else { 0x00 };
        data.push(pt_byte);
        
        data.extend_from_slice(&packet.sequence.to_be_bytes());
        data.extend_from_slice(&packet.timestamp.to_be_bytes());
        data.extend_from_slice(&packet.ssrc.to_be_bytes());
        
        data.extend_from_slice(&packet.payload);
        
        data
    }

    async fn send_to_subscriber(&self, data: &[u8], addr: SocketAddr) {
        let socket = self.socket
            .entry(addr)
            .or_insert_with(|| {
                let local_addr = if addr.is_ipv4() {
                    "0.0.0.0:0"
                } else {
                    "[::]:0"
                };
                
                match std::net::UdpSocket::bind(local_addr) {
                    Ok(udp_socket) => {
                        udp_socket.set_nonblocking(true).ok();
                        match UdpSocket::from_std(udp_socket) {
                            Ok(tokio_socket) => Arc::new(tokio_socket),
                            Err(_) => {
                                warn!("Failed to convert socket to tokio for {}", addr);
                                Arc::new(tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap())
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to bind socket for {}: {}", addr, e);
                        Arc::new(tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap())
                    }
                }
            });

        if let Err(e) = socket.send_to(data, addr).await {
            warn!("Failed to send packet to {}: {}", addr, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;

    #[tokio::test]
    async fn test_fanout_engine_creation() {
        let config = ServerConfig::default();
        let session_manager = Arc::new(SessionManager::new(config));
        let packet_queue = Arc::new(SegQueue::new());
        
        let engine = FanoutEngine::new(session_manager, packet_queue);
        assert!(engine.socket.is_empty());
    }
}
