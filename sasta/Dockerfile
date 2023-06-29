FROM --platform=$BUILDPLATFORM rust:slim as build
ENV PKG_CONFIG_ALLOW_CROSS=1 \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CC_armv7_unknown_Linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++

WORKDIR /usr/src/sasta
COPY . .

ARG TARGETARCH
ARG TARGETVARIANT

# Massive performance gains when building on `$BUILDPLATFORM` for `$TARGETPLATFORM` instead of
# on `$TARGETPLATFORM` all the way, so the added Dockerfile complexity is well worth it
RUN if [ "$TARGETARCH" = "arm" ] && [ "$TARGETVARIANT" = "v7" ];\
    then \
        apt update && apt upgrade -y; \
        apt install -y g++-arm-linux-gnueabihf libc6-dev-armhf-cross; \
        rustup target add armv7-unknown-linux-gnueabihf; \
        cargo build --target armv7-unknown-linux-gnueabihf --release; \
        mv target/armv7-unknown-linux-gnueabihf/release/sasta target/release/sasta;\
    else \
        cargo build --release; \
    fi

# Switch to Alpine instead for smaller images?
# FROM gcr.io/distroless/cc-debian10
FROM debian:stable-slim

COPY --from=build /usr/src/sasta/target/release/sasta /

CMD ["/sasta"]
