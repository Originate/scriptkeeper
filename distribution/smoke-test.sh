#!/usr/bin/env bash

set -eu

scriptkeeper=/root/scriptkeeper/distribution/scriptkeeper
test_script=/root/scriptkeeper/tests/examples/bigger/script

images=(
  centos:6.10
  centos:7.6.1810

  opensuse/leap:15.1
  opensuse/leap:42.3

  debian:squeeze
  debian:wheezy
  debian:jessie
  debian:stretch
  debian:buster

  fedora:26
  fedora:27
  fedora:28
  fedora:29

  ubuntu:14.04
  ubuntu:14.10
  ubuntu:15.04
  ubuntu:15.10
  ubuntu:16.04
  ubuntu:16.10
  ubuntu:17.04
  ubuntu:17.10
  ubuntu:18.04
  ubuntu:18.10
  ubuntu:19.04
)

for image in ${images[@]} ; do
  echo testing $image
  docker run --rm -i -v $(pwd):/root/scriptkeeper \
    --cap-add=SYS_PTRACE \
    $image $scriptkeeper $test_script
done
