FROM docker.io/rust:latest AS build

WORKDIR /sasta
COPY casta_protocol /casta_protocol
COPY sasta .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/sasta/target \
    cargo build --release && \
    cp /sasta/target/release/sasta /sasta/sasta_bin


FROM docker.io/debian:stable-slim
# TODO: LABEL env variables
RUN useradd -r sasta
WORKDIR /sasta

ENV FILE_PATH="/sasta/files"

COPY --from=build --chown=sasta:root /sasta/sasta_bin ./sasta
COPY --chown=sasta:root sasta/assets ./assets

RUN mkdir /sasta/files && chown sasta:root /sasta/files && chmod -R g=u /sasta

USER sasta
EXPOSE 8080

# Redis stores file metadata; file bytes live here.
# For multiple replicas, every pod must mount the same shared filesystem at FILE_PATH.
VOLUME ["/sasta/files"]

LABEL org.opencontainers.image.description="Sasta (Asta backend); multi-replica deployments require a shared filesystem mounted at FILE_PATH."


CMD ["./sasta"]
