FROM docker.io/rust:latest as build

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

ENV FILE_PATH="/files"

COPY --from=build --chown=sasta:root /sasta/sasta_bin ./sasta
COPY --chown=sasta:root sasta/assets ./assets

RUN mkdir /files && chown sasta:root /files && chmod -R g=u /files /sasta

USER sasta
EXPOSE 8080
VOLUME /sasta/files

CMD ["./sasta"]