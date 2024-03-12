FROM rust:latest AS builder
WORKDIR /gdqbot
COPY . .
RUN cargo build --release --target armv7-unknown-linux-musleabihf

FROM scratch
COPY --from=builder /gdqbot/target/armv7-unknown-linux-musleabihf/release/gdqbot /gdqbot/gdqbot
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
WORKDIR /gdqbot
ENTRYPOINT [ "./gdqbot" ]
