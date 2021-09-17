#!/bin/bash

set -x

checkout="${1:-main}"
cd ~/checkouts
git clone https://github.com/mozilla/glean
cd glean
git checkout "$checkout"

source taskcluster/scripts/rustup-setup.sh
echo "rust.targets=linux-x86-64\n" > local.properties

./gradlew clean assembleDebugUnitTest testDebugUnitTest
