FROM rust:latest AS build

WORKDIR /sasta
COPY casta_protocol /casta_protocol
COPY sasta .

RUN cargo build --release

FROM debian:stable

RUN groupadd -r sasta && useradd -r -g sasta sasta

RUN mkdir /files && chown sasta:sasta /files

WORKDIR /sasta

COPY --from=build --chown=sasta:sasta /sasta/target/release/sasta .
COPY --chown=sasta:sasta sasta/assets ./assets

USER sasta

EXPOSE 8080

CMD ["./sasta"]
