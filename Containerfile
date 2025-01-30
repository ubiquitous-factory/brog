FROM docker.io/fedora:41 as build

RUN dnf install -y gcc openssl-devel && \
    rm -rf /var/cache/dnf && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y

WORKDIR "/app-build"

ENV PATH=/root/.cargo/bin:${PATH}

COPY ./src ./src
COPY Cargo.toml  ./
RUN cargo build --release

FROM scratch 
COPY --from=build /app-build/target/release/brog ./
