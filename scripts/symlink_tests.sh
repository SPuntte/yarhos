#!/usr/bin/sh
# NOTE: assumes GNU sed

PACKAGE=$(cargo pkgid | sed -e "s|^.*/||" -e "s|#.*$||")
TARGET=$(cargo config get -Z unstable-options --format json-value build.target | sed -e 's/"//g' -e 's/\.json$//')
DEBUG_TARGET_DIR="target/$TARGET/debug"

# Retrieve test executable paths using cargo and symlink them in the debug target directory
cargo test --no-run 2>&1 | sed -e 's/^[[:blank:]]*//;s/[[:blank:]]*$//' -e '/Executable/!d' \
    -e "s|Executable unittests src/lib\.rs (.*/deps/\(.*\))|deps/\1 ${DEBUG_TARGET_DIR}/${PACKAGE}-unittests-lib|" \
    -e "s|Executable unittests src/main\.rs (.*/deps/\(.*\))|deps/\1 ${DEBUG_TARGET_DIR}/${PACKAGE}-unittests-bin|" \
    -e "s|Executable tests/.*\.rs (.*/deps/\(\(.*\)-[0-9a-f]*\))|deps/\1 ${DEBUG_TARGET_DIR}/${PACKAGE}-test-\2|" \
    | xargs -E "" -L 1 -t ln -sf