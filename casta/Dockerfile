FROM node:16-slim as build2
WORKDIR /usr/src/casta
COPY . .

RUN npm install && npm run build

FROM rust:1.65.0-slim as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/casta
COPY . .

RUN cargo build --release

# FROM gcr.io/distroless/cc-debian10
FROM debian:stable

COPY --from=build /usr/src/casta/target/release/casta /
COPY --from=build2 /usr/src/casta/static/index.html /static/index.html
COPY --from=build2 /usr/src/casta/static/target/index.js /static/target/index.js

CMD ["/casta"]
