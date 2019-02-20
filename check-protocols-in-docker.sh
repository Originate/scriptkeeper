#!/usr/bin/env bash

script=$1
if [[ "$script" = /* ]]; then
  docker run --rm -it --cap-add=SYS_PTRACE -v $script:/root/$(basename $script) -v $script.protocols.yaml:/root/$(basename $script).protocols.yaml check-protocols $(basename $script)
else
  echo please supply an absolute path
fi
