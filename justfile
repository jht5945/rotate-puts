_:
  @just --list

build-x64:
  cross build --target x86_64-unknown-linux-musl --release


