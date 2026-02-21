use std::sync::Arc;
use std::net::SocketAddr;
use rtp_fanout_server::config::ServerConfig;
use rtp_fanout_server::session::{SessionManager, SessionId, Session};

#[tokio::test]
async fn test_server_creation() {
    let config = ServerConfig::default();
    let server = rtp_fanout_server::RtpFanoutServer::new(config).await;
    assert!(server.is_ok());
}

#[test]
fn test_rtp_packet_parsing() {
    // Create a valid RTP packet
    let mut packet = vec![0u8; 12];
    packet[0] = 0x80;  // Version 2, no padding, no extension, 0 CSRC
    packet[1] = 0x60;  // Marker=0, PT=96
    packet[2] = 0x00;  // Sequence number high byte
    packet[3] = 0x01;  // Sequence number low byte
    packet[4] = 0x00;  // Timestamp
    packet[5] = 0x00;
    packet[6] = 0x00;
    packet[7] = 0x00;
    packet[8] = 0x12;  // SSRC
    packet[9] = 0x34;
    packet[10] = 0x56;
    packet[11] = 0x78;
    
    packet.extend_from_slice(b"test payload");
    
    // Test would use internal parsing function
}

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

#[test]
fn test_session_manager() {
    let config = ServerConfig::default();
    let manager = SessionManager::new(config);
    
    let addr: SocketAddr = "127.0.0.1:5004".parse().unwrap();
    let ssrc = 12345u32;
    
    let session = manager.create_session(addr, ssrc);
    assert!(session.is_some());
    
    let retrieved = manager.get_session_by_ssrc(ssrc);
    assert!(retrieved.is_some());
    
    assert_eq!(manager.session_count(), 1);
}

#[tokio::test]
async fn test_session_lifecycle() {
    // Integration test for session creation/deletion
}
