ARG RUST_VERSION=1.91.1

FROM rust:${RUST_VERSION}-slim-trixie as build
WORKDIR /tmp/build
# COPY Cargo.lock Cargo.toml /tmp/build/
COPY Cargo.toml /tmp/build/
RUN mkdir -p /tmp/build/src && \
  echo "fn main() {}" > /tmp/build/src/main.rs

COPY rusty_bot_macros/Cargo.toml /tmp/build/rusty_bot_macros/Cargo.toml
RUN mkdir -p /tmp/build/rusty_bot_macros/src && \
  touch /tmp/build/rusty_bot_macros/src/lib.rs
RUN cargo fetch
RUN cargo build --release

# Dependencies are now cached, copy the actual source code and do another full
# build. The touch on all the .rs files is needed, otherwise cargo assumes the
# source code didn't change thanks to mtime weirdness.
RUN rm -rf /tmp/build/src
COPY src /tmp/build/src
RUN find -name "*.rs" -exec touch {} \; && cargo build --release

##################
#  Output image  #
##################
FROM debian:trixie-slim
COPY --from=build /tmp/build/target/release/rusty-bot /usr/local/bin/

ENV RUST_LOG=info
CMD rusty_bot