#!/usr/bin/env bash

script=$1
if [[ "$script" = /* ]]; then
  true
else
  script=$(pwd)/$script
fi

docker run --rm -it --cap-add=SYS_PTRACE \
  --mount type=bind,source=$script,target=/root/$(basename $script) \
  --mount type=bind,source=$script.test.yaml,target=/root/$(basename $script).test.yaml \
  scriptkeeper $(basename $script)
