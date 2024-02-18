FROM messense/rust-musl-cross:armv7-musleabihf AS builder
WORKDIR /gdqbot
RUN apt update && apt-get -y upgrade && apt-get -y install ca-certificates
RUN update-ca-certificates
COPY . .
RUN cargo build --release --target armv7-unknown-linux-musleabihf

FROM scratch
COPY --from=builder /gdqbot/target/armv7-unknown-linux-musleabihf/release/gdqbot /gdqbot/gdqbot
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
WORKDIR /gdqbot
ENTRYPOINT [ "./gdqbot" ]
