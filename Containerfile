FROM docker.io/fedora@sha256:d0207dbb078ee261852590b9a8f1ab1f8320547be79a2f39af9f3d23db33735e as build

RUN dnf install -y gcc openssl-devel && \
    rm -rf /var/cache/dnf && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y

WORKDIR "/app-build"

ENV PATH=/root/.cargo/bin:${PATH}

COPY ./src ./src
COPY Cargo.toml  ./
RUN cargo build --release

FROM docker.io/fedora@sha256:d0207dbb078ee261852590b9a8f1ab1f8320547be79a2f39af9f3d23db33735e

ENV container docker
RUN dnf -y update; dnf clean all
RUN dnf -y install openssl-devel; dnf clean all;
WORKDIR "/app"
COPY --from=build /app-build/target/release/brog ./

CMD [ "./brog" ]