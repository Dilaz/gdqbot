FROM rust:1.85 AS builder
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /gdqbot
RUN git clone --depth 1 https://github.com/dilaz/kvstore /kvstore
COPY . .
RUN mkdir -p .cargo && echo '[patch."https://github.com/dilaz/kvstore"]' > .cargo/config.toml && \
    echo 'kvstore-client = { path = "/kvstore/kvstore-client" }' >> .cargo/config.toml && \
    cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot
ENTRYPOINT [ "/gdqbot" ]
