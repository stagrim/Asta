FROM rust:latest as build

WORKDIR /sasta
COPY casta_protocol /casta_protocol
COPY sasta .

RUN cargo build --release

# Switch to Alpine instead for smaller images?
# FROM gcr.io/distroless/cc-debian10
FROM debian:stable-slim
WORKDIR /sasta

COPY --from=build /sasta/target/release/sasta /sasta/
COPY sasta/assets /sasta/assets

VOLUME /sasta/files
CMD ["/sasta/sasta"]
