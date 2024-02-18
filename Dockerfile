FROM messense/rust-musl-cross:armv7-musleabihf AS builder
WORKDIR /gdqbot
COPY . .
RUN cargo build --release --target armv7-unknown-linux-musleabihf

FROM scratch
COPY --from=builder /gdqbot/target/armv7-unknown-linux-musleabihf/release/gdqbot /gdqbot/gdqbot
WORKDIR /gdqbot
ENTRYPOINT [ "./gdqbot" ]
