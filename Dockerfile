FROM rust:1.32.0

ADD Cargo.* /root/check-protocols/
ADD src /root/check-protocols/src
WORKDIR /root/check-protocols
RUN cargo install --root /usr/local --path .
WORKDIR /root
RUN rm /root/check-protocols -rf

ENTRYPOINT ["check-protocols"]
