FROM centos:6.10
RUN yum install --assumeyes gcc

# install rust toolchain
WORKDIR /root
RUN curl https://sh.rustup.rs -sSf >> rustup.sh
RUN chmod +x rustup.sh
ARG RUSTC_VERSION
RUN ./rustup.sh -y --default-toolchain $RUSTC_VERSION
ENV PATH=/root/.cargo/bin:$PATH

# build scriptkeeper
WORKDIR /root/scriptkeeper
ADD Cargo.* ./
RUN mkdir src && touch src/lib.rs && cargo install --root /usr/local --path . ; true
ADD src ./src
RUN touch src/lib.rs
RUN cargo install --root /usr/local --path .
RUN strip /usr/local/bin/scriptkeeper
