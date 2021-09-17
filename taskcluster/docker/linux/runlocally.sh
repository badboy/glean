#!/bin/bash

# Run the Android build in Docker.
# This should run exactly the same as the Taskcluster build, using a local Docker setup.
# It only runs the Android build & test.
# The Docker image is rebuild everytime (Docker is smart enough to skip cached steps though).
# Note: the Docker image is 15 GB.

WORKSPACE_ROOT="$( cd "$(dirname "$0")" ; pwd -P )"
cd "${WORKSPACE_ROOT}"

# Build docker image to use
docker build -t gleanlinux .

# Run the Android build
docker run --rm -v "$(pwd):/glean" gleanlinux /glean/runbuild.sh
