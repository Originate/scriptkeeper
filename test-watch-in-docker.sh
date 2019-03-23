#!/usr/bin/env bash

docker run --init --rm -it --cap-add=SYS_PTRACE \
  --entrypoint "/bin/bash" -v $(pwd):/root/ \
  scriptkeeper -c "DEBUG=1 cargo watch -x 'test can_dir_glob --features test -- --test-threads=1 --quiet'"
