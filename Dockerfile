FROM rust:1.24-stretch as builder

ADD . ./

RUN apt update
RUN apt install -y libssl-dev
RUN cargo build --verbose --release
RUN cargo install

FROM debian:stretch
COPY --from=builder /usr/local/cargo/bin/rs_watch /usr/bin

RUN apt update && apt install -y libssl1.1 ca-certificates
CMD rs_watch
