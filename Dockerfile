FROM rust:1.85 AS builder
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /gdqbot
COPY . .
RUN rm -rf /usr/local/cargo/git/db/kvstore-* /usr/local/cargo/git/checkouts/kvstore-* && \
    cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot
ENTRYPOINT [ "/gdqbot" ]
