#!/usr/bin/env bash

set -eu

image=scriptkeeper-distribution
container=$image-container

docker build --file distribution/Dockerfile . --tag scriptkeeper-distribution

docker run --name $container $image true
docker cp $container:/usr/local/bin/scriptkeeper distribution/
docker rm $container
