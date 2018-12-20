FROM rust:1.31
RUN apt-get update && apt-get install -y cmake libgit2-dev musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get install -y clang libclang-dev
RUN cargo install bindgen
RUN rustup component add rustfmt-preview
