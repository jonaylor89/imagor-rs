FROM rust:latest AS builder

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y libvips-dev && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

COPY src src
COPY samples samples

RUN cargo build --release --bin imagor-rs

FROM debian:bookworm-slim
LABEL maintainer="jonaylor89@gmail.com"

RUN DEBIAN_FRONTEND=noninteractive \
    apt-get update && \
    apt-get install --no-install-recommends -y \
    ca-certificates \
    libvips \
    openssl \
    libssl3 \
    pkg-config \
    curl \
    procps libglib2.0-0 libjpeg62-turbo libpng16-16 libopenexr-3-1-30 \
    libwebp7 libwebpmux3 libwebpdemux2 libtiff6 libexif12 libxml2 libpoppler-glib8 \
    libpango1.0-0 libmatio11 libopenslide0 libopenjp2-7 libjemalloc2 \
    libgsf-1-114 libfftw3-bin liborc-0.4-0 librsvg2-2 libcfitsio10 libimagequant0 libaom3 libheif1 \
    libspng0 libcgif0 && \
    ln -s /usr/lib/$(uname -m)-linux-gnu/libjemalloc.so.2 /usr/local/lib/libjemalloc.so && \
    apt-get autoremove -y && \
    apt-get autoclean && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

COPY --from=builder /usr/src/app/target/release/imagor-rs /usr/local/bin/imagor-rs
COPY samples samples

ENV RUST_LOG=info
ENV SSL_CERT_DIR=/etc/ssl/certs
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

# use unprivileged user
USER nobody

EXPOSE 8080

CMD ["imagor-rs"]
