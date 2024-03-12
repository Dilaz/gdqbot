FROM rust:latest AS builder
WORKDIR /gdqbot
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/static-debian12
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot/gdqbot
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
WORKDIR /gdqbot
ENTRYPOINT [ "./gdqbot" ]
