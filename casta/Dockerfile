FROM rust:1.63.0 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/casta
COPY . .

RUN cargo install --path .

FROM gcr.io/distroless/cc-debian10
# FROM debian:stable

COPY --from=build /usr/local/cargo/bin/casta /usr/local/bin/casta
COPY --from=build /usr/src/casta/static/index.html /static/index.html

CMD ["casta"]
