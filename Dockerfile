FROM rust:latest AS builder
WORKDIR /gdqbot
COPY . .
RUN cargo build --release

FROM scratch
COPY --from=builder /gdqbot/target/release/gdqbot /gdqbot/gdqbot
WORKDIR /gdqbot
ENTRYPOINT [ "./gdqbot" ]
