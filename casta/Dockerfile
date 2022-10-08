FROM node:16 as build2
WORKDIR /usr/src/casta
COPY . .

RUN npm install && npm run build

FROM rust:1.63.0 as build
ENV PKG_CONFIG_ALLOW_CROSS=1

WORKDIR /usr/src/casta
COPY . .

RUN cargo install --path .

FROM gcr.io/distroless/cc-debian10
# FROM debian:stable

COPY --from=build /usr/local/cargo/bin/casta /usr/local/bin/casta
COPY --from=build /usr/src/casta/static/index.html /static/index.html
COPY --from=build2 /usr/src/casta/static/target/index.js /static/target/index.js

CMD ["casta"]
