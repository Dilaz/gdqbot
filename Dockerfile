FROM rust:1.83 AS builder
WORKDIR /gdqbot
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot
ENTRYPOINT [ "/gdqbot" ]
