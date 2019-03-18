ci: test build fmt clippy doc scripts

test:
  cargo test --all --color=always --features 'ci test' -- --test-threads=1 --quiet

build:
  cargo build --features=ci

fmt:
  cargo fmt -- --check

clippy:
  cargo clippy --tests --color=always --features 'ci test'

doc:
  cargo doc

scripts:
  cargo run -- build-docker-image.sh
  cargo run -- check-protocols-in-docker.sh

dev:
  clear ; printf "\e[3J"
  cargo test --all --color=always --features 'dev test' -- --test-threads=1

run_bigger:
  cargo run -- tests/examples/bigger/script

test_dockerfile:
  docker build -t check-protocols .
  docker run --rm \
    --cap-add=SYS_PTRACE \
    -v $(pwd)/tests/examples/bigger/script:/root/script \
    -v $(pwd)/tests/examples/bigger/script.protocols.yaml:/root/script.protocols.yaml \
    check-protocols \
    script
