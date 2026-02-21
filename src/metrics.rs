use metrics::{counter, gauge, histogram};
use std::time::Instant;
use tracing::info;

pub struct MetricsCollector;

impl MetricsCollector {
    pub fn init() {
        if let Err(e) = metrics_exporter_prometheus::PrometheusBuilder::new()
            .install_recorder() {
            tracing::warn!("Failed to install Prometheus recorder: {}", e);
        }
    }

    pub fn record_packet_received(size: usize) {
        counter!("rtp_packets_received_total").increment(1);
        counter!("rtp_bytes_received_total").increment(size as u64);
    }

    pub fn record_packet_sent(subscriber_count: usize) {
        counter!("rtp_packets_sent_total").increment(subscriber_count as u64);
    }

    pub fn record_fanout_latency(latency_ms: f64) {
        histogram!("fanout_latency_ms").record(latency_ms);
    }

    pub fn update_session_count(count: usize) {
        gauge!("active_sessions").set(count as f64);
    }

    pub fn update_subscriber_count(count: usize) {
        gauge!("total_subscribers").set(count as f64);
    }
}
