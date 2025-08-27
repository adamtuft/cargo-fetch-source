#! /usr/bin/env bash

# Reads commit hashes from stdin and filters for those which do not match the conventional commit
# format

CONVENTIONAL_COMMIT_REGEX='^(?<sha>[0-9a-f]{7,40}) (?<type>build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(?<scope>\(\w+\)?((?=:\s)|(?=!:\s)))?(?<breaking>!)?(?<subject>:\s.*)?|^(?<merge>Merge \w+)'

# grep -P -v "$CONVENTIONAL_COMMIT_REGEX" <(xargs -I {} git rev-list -n 1 --pretty=oneline {})
xargs -I {} git rev-list -n 1 --pretty=oneline {} | grep -P -v "$CONVENTIONAL_COMMIT_REGEX"
