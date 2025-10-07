FROM rust:1.85 AS builder
WORKDIR /usr/local/app/
RUN apt-get update && apt-get install -y cmake pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

ARG S3_ACCESS_KEY_ID
ARG S3_SECRET_ACCESS_KEY
ARG BIN_NAME

# First, we want to copy the initial dependency files so docker can cache them
COPY Cargo.toml Cargo.lock ./

COPY crates/img-api-server/Cargo.toml crates/img-api-server/
COPY crates/consumers/Cargo.toml crates/consumers/


RUN mkdir crates/${BIN_NAME}/src && echo "fn main() {}" > crates/${BIN_NAME}/src/main.rs

RUN ls -a
RUN cargo build --release --workspace || true


# This copy doesnt create duplicates because docker will overwrite everything
COPY . .

RUN cargo build --release --bin ${BIN_NAME}
RUN cp target/release/${BIN_NAME} target/release/app

FROM debian:bookworm-slim
WORKDIR /usr/local/app/
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder  usr/local/app/target/release/app /usr/local/bin/app
CMD ["app"]
EXPOSE 3030


