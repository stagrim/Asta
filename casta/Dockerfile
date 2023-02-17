FROM --platform=$BUILDPLATFORM rust:1.65.0-slim as build
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN apt update && apt upgrade -y 
RUN apt install -y g++-arm-linux-gnueabihf libc6-dev-armhf-cross

RUN rustup target add armv7-unknown-linux-gnueabihf 
RUN rustup toolchain install stable-armv7-unknown-linux-gnueabihf

WORKDIR /usr/src/casta
COPY . .

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc CC_armv7_unknown_Linux_gnueabihf=arm-linux-gnueabihf-gcc CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++

RUN cargo build --target armv7-unknown-linux-gnueabihf --release

FROM --platform=$BUILDPLATFORM node:16-slim as build2
WORKDIR /usr/src/casta
COPY . .

# `|| :` added to ignore errors...
RUN npm install && npm run build || :

# FROM gcr.io/distroless/cc-debian10
FROM debian:stable-slim

COPY --from=build /usr/src/casta/target/armv7-unknown-linux-gnueabihf/release/casta /
COPY --from=build2 /usr/src/casta/static/index.html /static/index.html
COPY --from=build2 /usr/src/casta/static/target/index.js /static/target/index.js
COPY --from=build2 /usr/src/casta/static/disconnect.png /static/disconnect.png

CMD ["/casta"]
