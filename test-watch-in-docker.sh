#!/usr/bin/env bash

docker run --init --rm -it --cap-add=SYS_PTRACE \
  --entrypoint "/bin/bash" -v $(pwd):/root/ \
  scriptkeeper -c "cargo watch -x 'test --features test -- --test-threads=1 --quiet'"
