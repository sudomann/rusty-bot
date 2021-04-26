FROM ubuntu:21.04

RUN apt-get update && apt-get install -y \
  curl \
  build-essential \
  && rm -rf /var/lib/apt/lists/* \
  && curl https://sh.rustup.rs -sSf | sh -s -- -y

COPY ["src/", "/app/src/"]
COPY ["Cargo.toml", "/app/"]
WORKDIR /app
RUN ["/bin/bash", "-c", "source $HOME/.cargo/env && cargo build --release"]
COPY [".env", "."]
RUN rm -rf target/release/build/ \
  target/release/deps/ \
  target/release/examples/ \
  target/release/incremental/

CMD target/release/rusty-bot
