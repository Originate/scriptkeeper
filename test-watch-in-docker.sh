#!/usr/bin/env bash

docker run --init --rm -it --cap-add=SYS_PTRACE \
  --entrypoint "/bin/bash" -v $(pwd):/root/check-protocols \
  check-protocols -c "(cd check-protocols; cargo watch -x 'test -- --test-threads=1 --quiet')"
