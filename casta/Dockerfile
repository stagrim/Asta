FROM node:16 as build2
WORKDIR /usr/src/casta
COPY . .

RUN npm install && npm run build

FROM rust:1.65.0 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/casta
COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc-debian10
# FROM debian:stable

COPY --from=build ./target/release/casta --from=build2 /usr/src/casta/static/ ./

CMD ["casta"]
