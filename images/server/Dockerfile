# AnyBot server image (spec/06 S4, spec/09 K110).
# Build stage: official rust 1.98.1-slim-bookworm.
# Multi-arch index digest (linux/amd64 + linux/arm64) queried from registry-1.docker.io on 2026-11-25:
#   rust:1.98.1-slim-bookworm
#   index sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730
# Runtime stage: official debian 13.2-slim (Debian 13.2 point release, trixie).
#   debian:13.2-slim
#   index sha256:4bcb9db66237237d03b55b969271728dd3d955eaaa254b9db8a3db94550b1885
# Config comes only from --bootstrap. This image sets no ANYBOT_* environment variables.

FROM rust:1.98.1-slim-bookworm@sha256:ff521445a372125ed4f76e1453a1f8098f2d05332d1601d30db1c1f62757e730 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY apps ./apps
COPY tools ./tools
COPY tests ./tests
COPY locales ./locales
ARG ANYBOT_RELEASE_CHANNEL=stable
ARG ANYBOT_GIT_DESCRIBE=unknown
ARG SOURCE_DATE_EPOCH=0
ENV ANYBOT_RELEASE_CHANNEL=${ANYBOT_RELEASE_CHANNEL} \
    ANYBOT_GIT_DESCRIBE=${ANYBOT_GIT_DESCRIBE} \
    SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH}
RUN cargo build --locked --release --bin anybot-server

FROM debian:13.2-slim@sha256:4bcb9db66237237d03b55b969271728dd3d955eaaa254b9db8a3db94550b1885
RUN useradd --system --create-home --home-dir /var/lib/anybot --shell /usr/sbin/nologin anybot
COPY --from=build /src/target/release/anybot-server /usr/local/bin/anybot-server
USER anybot
WORKDIR /var/lib/anybot
ENTRYPOINT ["/usr/local/bin/anybot-server"]
CMD ["--version"]
