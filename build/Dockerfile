FROM rust:1.83-slim-bookworm as builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY proto ./proto

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rtp-fanout-server /usr/local/bin/
COPY config/server.toml /etc/rtp-fanout/config.toml

EXPOSE 5004/udp 9090/tcp

ENV RUST_LOG=info
ENV RTP_FANOUT__BIND_ADDRESS=0.0.0.0:5004

CMD ["rtp-fanout-server"]
