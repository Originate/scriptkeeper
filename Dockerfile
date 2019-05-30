ARG RUSTC_VERSION
FROM rust:${RUSTC_VERSION}

RUN cargo install cargo-watch
RUN apt-get update && apt-get install -y ruby

WORKDIR /root/scriptkeeper
ADD Cargo.* ./
RUN mkdir src && touch src/lib.rs && cargo install --root /usr/local --path . ; true
ADD src ./src
RUN touch src/lib.rs
RUN cargo install --root /usr/local --path .
WORKDIR /root
RUN rm /root/scriptkeeper -rf

ENTRYPOINT ["scriptkeeper"]
