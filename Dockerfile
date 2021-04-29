FROM rust:1.51.0-alpine3.13

RUN apk add --update alpine-sdk
COPY ["src/", "/app/src/"]
COPY ["Cargo.toml", "/app/"]
WORKDIR /app
RUN cargo build --release


RUN rm -rf target/release/build/ \
  target/release/deps/ \
  target/release/examples/ \
  target/release/incremental/

RUN --mount=type=secret,id=dotenv cat /run/secrets/dotenv
#ARG dotenv
#COPY ${dotenv} .
#RUN echo ${dotenv} > .env

# ENTRYPOINT [ "target/release/rusty-bot" ]
CMD [ "target/release/rusty-bot" ]