FROM rust:1.32.0

RUN cargo install cargo-watch

WORKDIR /root/check-protocols
ADD Cargo.* ./
RUN mkdir src && touch src/lib.rs && cargo install --root /usr/local --path . ; true
ADD src ./src
RUN touch src/lib.rs
RUN cargo install --root /usr/local --path .
WORKDIR /root
RUN rm /root/check-protocols -rf

ENTRYPOINT ["check-protocols"]
