FROM rust:1.27

ADD . .

RUN apt update && \
    apt install -y libssl-dev && \
    cargo build --verbose --release && \
    cargo install

ENV PATH "$PATH:/root/.cargo/bin/"
CMD rs_watch
