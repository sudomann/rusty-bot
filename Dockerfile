FROM ubuntu:21.04

RUN apt-get update && apt-get install -y \
  curl \
  && rm -rf /var/lib/apt/lists/* \
  && curl https://sh.rustup.rs -sSf | sh -s -- -y

COPY ["src/", "/app/src/"]
COPY ["Cargo.toml", "/app/"]
WORKDIR /app
RUN ["/bin/bash", "-c", "source $HOME/.cargo/env && cargo build --release"]

RUN rm -rf target/rls/build/ \
  target/rls/deps/ \
  target/rls/examples/ \
  target/rls/incremental/

CMD target/rls/rusty-bot.sh
