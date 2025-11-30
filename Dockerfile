FROM rust:1.85 AS builder
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /gdqbot
RUN git clone --bare https://github.com/dilaz/kvstore /usr/local/cargo/git/db/kvstore-24845053cd73a984
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot
ENTRYPOINT [ "/gdqbot" ]
