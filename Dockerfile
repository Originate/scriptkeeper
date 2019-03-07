FROM ubuntu:18.04

RUN apt-get update && apt-get install -y curl
RUN curl https://sh.rustup.rs -sSf >> installer
RUN chmod +x installer
RUN ./installer -y
ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y build-essential
RUN cargo install cargo-watch
RUN apt-get update && apt-get install -y ruby

WORKDIR /root/check-protocols
ADD Cargo.* ./
RUN mkdir src && touch src/lib.rs && cargo install --root /usr/local --path . ; true
ADD src ./src
RUN touch src/lib.rs
RUN cargo install --root /usr/local --path .
WORKDIR /root
RUN rm /root/check-protocols -rf
RUN ls /root/.cargo/bin

ENTRYPOINT ["check-protocols"]
