pub mod config;
pub mod session;
pub mod fanout;
pub mod metrics;

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use dashmap::DashMap;
use crossbeam::queue::SegQueue;

use config::ServerConfig;
use session::{SessionManager, SessionId};
use fanout::FanoutEngine;

#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub payload: Vec<u8>,
    pub timestamp: u32,
    pub sequence: u16,
    pub ssrc: u32,
    pub marker: bool,
}

pub struct RtpFanoutServer {
    config: ServerConfig,
    socket: Arc<UdpSocket>,
    session_manager: Arc<SessionManager>,
    fanout_engine: Arc<FanoutEngine>,
    packet_queue: Arc<SegQueue<RtpPacket>>,
}

impl RtpFanoutServer {
    pub async fn new(config: ServerConfig) -> anyhow::Result<Self> {
        let bind_addr: SocketAddr = config.bind_address.parse()?;
        let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
        info!("RTP server binding to {}", bind_addr);

        let session_manager = Arc::new(SessionManager::new(config.clone()));
        let packet_queue = Arc::new(SegQueue::new());
        let fanout_engine = Arc::new(FanoutEngine::new(
            session_manager.clone(),
            packet_queue.clone(),
        ));

        Ok(Self {
            config,
            socket,
            session_manager,
            fanout_engine,
            packet_queue,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Starting RTP Fanout Server v{}", env!("CARGO_PKG_VERSION"));
        
        let mut buf = vec![0u8; 65535];
        
        loop {
            match self.socket.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    if let Some(packet) = Self::parse_rtp_packet(&buf[..len]) {
                        self.handle_packet(packet, addr).await;
                    }
                }
                Err(e) => {
                    error!("UDP receive error: {}", e);
                }
            }
        }
    }

    fn parse_rtp_packet(data: &[u8]) -> Option<RtpPacket> {
        if data.len() < 12 {
            return None;
        }

        let version = (data[0] >> 6) & 0x03;
        if version != 2 {
            return None;
        }

        let padding = (data[0] >> 5) & 0x01;
        let extension = (data[0] >> 4) & 0x01;
        let csrc_count = data[0] & 0x0F;
        let marker = ((data[1] >> 7) & 0x01) != 0;
        
        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let header_len = 12 + (csrc_count as usize * 4);
        let mut payload_start = header_len;

        if extension != 0 {
            if data.len() < header_len + 4 {
                return None;
            }
            let ext_len = u16::from_be_bytes([data[header_len + 2], data[header_len + 3]]) as usize;
            payload_start += 4 + (ext_len * 4);
        }

        let mut payload_end = data.len();
        if padding != 0 && !data.is_empty() {
            let padding_len = data[data.len() - 1] as usize;
            if padding_len > 0 && padding_len <= data.len() - payload_start {
                payload_end -= padding_len;
            }
        }

        Some(RtpPacket {
            payload: data[payload_start..payload_end].to_vec(),
            timestamp,
            sequence,
            ssrc,
            marker,
        })
    }

    async fn handle_packet(&self, packet: RtpPacket, addr: SocketAddr) {
        debug!("Received RTP packet from {}: ssrc={}, seq={}, ts={}", 
               addr, packet.ssrc, packet.sequence, packet.timestamp);
        
        self.packet_queue.push(packet);
        self.fanout_engine.process_batch().await;
    }
}
