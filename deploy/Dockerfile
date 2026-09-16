ARG BUILDER_IMAGE=rust:1.93.0-alpine3.23@sha256:6f37c2b56fde862a67116c4bddbb6014c0a758b24944f8650c8ff8c735f96f06
ARG RUNTIME_IMAGE=gcr.io/distroless/static-debian13:nonroot@sha256:2293b36c7c9082bf4115aab724b4d2cddec82c8eba39bf27ac0517e159acf150
FROM ${BUILDER_IMAGE} AS build
RUN apk add --no-cache build-base protoc protobuf-dev pkgconf perl linux-headers cmake
WORKDIR /source
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY schemas ./schemas
COPY prompts ./prompts
ENV SOURCE_DATE_EPOCH=0 CARGO_INCREMENTAL=0 RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static --remap-path-prefix=/source=."
RUN cargo build --locked --release --target x86_64-unknown-linux-musl -p hm-cli --features portable && ! readelf -l target/x86_64-unknown-linux-musl/release/hm | grep 'Requesting program interpreter'
RUN mkdir -p /state && chown 65532:65532 /state
FROM ${RUNTIME_IMAGE}
ARG HM_RUNTIME_UID=65532
ARG HM_RUNTIME_GID=65532
COPY --from=build /source/target/x86_64-unknown-linux-musl/release/hm /usr/local/bin/hm
COPY --from=build --chown=65532:65532 /state /var/lib/hypermind
COPY models/manifest.toml /usr/share/hypermind/models.toml
USER ${HM_RUNTIME_UID}:${HM_RUNTIME_GID}
WORKDIR /var/lib/hypermind
STOPSIGNAL SIGINT
ENTRYPOINT ["/usr/local/bin/hm"]
CMD ["serve", "--config", "/var/lib/hypermind/hypermind.conf"]
