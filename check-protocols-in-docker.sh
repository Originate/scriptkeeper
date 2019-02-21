#!/usr/bin/env bash

script=$1
if [[ "$script" = /* ]]; then
  docker run --rm -it --cap-add=SYS_PTRACE \
    --mount type=bind,source=$script,target=/root/$(basename $script) \
    --mount type=bind,source=$script.protocols.yaml,target=/root/$(basename $script).protocols.yaml \
    check-protocols $(basename $script)
else
  echo please supply an absolute path
fi
