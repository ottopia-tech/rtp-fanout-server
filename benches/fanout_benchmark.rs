use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::sync::Arc;
use crossbeam::queue::SegQueue;

fn benchmark_packet_queue(c: &mut Criterion) {
    let queue = Arc::new(SegQueue::<Vec<u8>>::new());
    
    let mut group = c.benchmark_group("packet_queue");
    group.throughput(Throughput::Elements(1000));
    
    group.bench_function("push_pop", |b| {
        b.iter(|| {
            for i in 0..1000 {
                queue.push(black_box(vec![i as u8; 1400]));
            }
            for _ in 0..1000 {
                black_box(queue.pop());
            }
        });
    });
    
    group.finish();
}

fn benchmark_rtp_parsing(c: &mut Criterion) {
    let packet = create_test_rtp_packet();
    
    c.bench_function("rtp_parse", |b| {
        b.iter(|| {
            black_box(parse_rtp_packet(&packet));
        });
    });
}

fn create_test_rtp_packet() -> Vec<u8> {
    let mut packet = vec![0u8; 1400];
    packet[0] = 0x80;  // Version 2
    packet[1] = 0x60;  // PT=96
    packet[2..4].copy_from_slice(&12345u16.to_be_bytes());
    packet[4..8].copy_from_slice(&987654321u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0xDEADBEEFu32.to_be_bytes());
    packet
}

fn parse_rtp_packet(data: &[u8]) -> Option<(u16, u32, u32)> {
    if data.len() < 12 {
        return None;
    }
    
    let version = (data[0] >> 6) & 0x03;
    if version != 2 {
        return None;
    }
    
    let sequence = u16::from_be_bytes([data[2], data[3]]);
    let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    
    Some((sequence, timestamp, ssrc))
}

criterion_group!(benches, benchmark_packet_queue, benchmark_rtp_parsing);
criterion_main!(benches);
