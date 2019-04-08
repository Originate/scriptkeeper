ci: test build fmt clippy doc scripts

test:
  cargo test -p test-utils --color=always -- --test-threads=1 --quiet
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
  cargo run -- scriptkeeper-in-docker.sh
  cargo run -- distribution/build.sh

dev pattern='':
  clear ; printf "\e[3J"
  cargo test -p test-utils --color=always -- --test-threads=1 --quiet
  cargo test --all --color=always --features 'dev test' -- --test-threads=1 --quiet {{pattern}}

run_bigger:
  cargo run -- tests/examples/bigger/script

export RUSTC_VERSION := "1.34.2"

build_docker_image:
  docker build --build-arg RUSTC_VERSION -t scriptkeeper .

test_dockerfile: build_docker_image
  docker run --rm \
    --cap-add=SYS_PTRACE \
    -v $(pwd)/tests/examples/bigger/script:/root/script \
    -v $(pwd)/tests/examples/bigger/script.test.yaml:/root/script.test.yaml \
    scriptkeeper \
    script

distribution_smoke_test: distribution_build
  ./distribution/smoke-test.sh

distribution_build:
  ./distribution/build.sh
