#!/usr/bin/env bash

set -euo pipefail

if [[ $(git symbolic-ref --short HEAD) != "master" ]] ; then
  echo not on master, aborting... 1>&2
  exit 1
fi

if [[ -n $(git status --porcelain) ]] ; then
  echo git repo not clean, aborting... 1>&2
  exit 1
fi

cargo bump
