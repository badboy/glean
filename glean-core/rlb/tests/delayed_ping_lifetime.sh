#!/bin/bash

# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# This integration test model how the RLB is used when embedded in another Rust application
# (e.g. FOG/Firefox Desktop).
# 
# It is implemented as a shell script to be able to "restart" the application by just calling it again.
# The example application implements some logic that is triggered by different arugments.

set -e

build_example() {
    cargo build --package glean --example prototype
}

run_example() {
  cargo run --quiet --package glean --example prototype -- $*
}

THIS_DIR="$( cd "$(dirname "$0")" ; pwd -P )"
TMPDIR="${TMPDIR:-/tmp}"
DATA_DIR="$(mktemp -d "${TMPDIR}/glean_prototype.$$.XXXXXX")"
PENDING_PINGS_DIRECTORY="${DATA_DIR}/pending_pings"

cd "${THIS_DIR}"
echo "Data directory: ${DATA_DIR}"

# Log everything
export RUST_LOG=debug

# Build it first
build_example

# First run does not submit a ping, but records data.
echo "==== First run ===="
run_example "${DATA_DIR}" delay skip
# Remove the initial metrics ping, we don't care about it.
rm "${PENDING_PINGS_DIRECTORY}"/*
# Second run will record data, then submit.
echo "==== Second run ===="
run_example "${DATA_DIR}" delay submit

pingfile=$(find "${PENDING_PINGS_DIRECTORY}" -type f)
payload=$(tail -1 "${pingfile}")
counter=$(echo "$payload" | jq '.metrics.counter."test.metrics.sample_counter"')

if [ "$counter" -eq 2 ]; then
    printf "\n\033[1;37m\\o/ \033[0;32mTest passed without errors!\033[0m\n"
    rm -rf "${DATA_DIR}"
    exit 0
else
    printf "\n\033[1;37m/o\ \033[0;31mTest failed!\033[0m\n"

    echo "Received payload:"
    echo "$payload" | jq .

    exit 1
fi
