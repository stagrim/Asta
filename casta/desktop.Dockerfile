# Also works to compile for arm, but is much slower since compilation is done in qemu. This dockerfile should therefor only be used when compiling for x86

# fetch the vendor with the builder platform to avoid qemu issues (https://github.com/rust-lang/cargo/issues/9545#issuecomment-855282576)
FROM --platform=$BUILDPLATFORM rust:1.65-slim AS server-sources

ENV USER=root

WORKDIR /code
RUN cargo init
COPY ./Cargo.toml /code/Cargo.toml
COPY ./Cargo.lock /code/Cargo.lock
RUN mkdir -p /code/.cargo \
  && cargo vendor > /code/.cargo/config

FROM rust:1.65.0-slim as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/casta
COPY . .
COPY --from=server-sources /code/.cargo ./.cargo
COPY --from=server-sources /code/vendor ./vendor

RUN cargo build --release --offline

FROM node:16-slim as build2
WORKDIR /usr/src/casta
COPY . .

RUN npm install && npm run build

# FROM gcr.io/distroless/cc-debian10
FROM debian:stable

COPY --from=build /usr/src/casta/target/release/casta /
COPY --from=build2 /usr/src/casta/static/index.html /static/index.html
COPY --from=build2 /usr/src/casta/static/target/index.js /static/target/index.js
COPY --from=build2 /usr/src/casta/static/disconnect.png /static/disconnect.png

CMD ["/casta"]
