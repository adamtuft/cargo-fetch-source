#! /usr/bin/env bash

printf " ==== RUN: %s\n" "$0"

cargo clippy --all-targets --all-features
