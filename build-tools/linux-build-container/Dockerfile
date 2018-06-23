FROM rust:1.26.2
RUN apt-get update && apt-get install -y cmake libgit2-dev musl-tools
RUN rustup target add x86_64-unknown-linux-musl
