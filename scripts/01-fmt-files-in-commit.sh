#! /usr/bin/env bash

set -euo pipefail

printf " ==== RUN: %s\n" "$0"
readarray -t COMMIT_FILES_RS < <( git --no-pager diff --name-only --cached | grep ".rs$" )
NUM_COMITTED=${#COMMIT_FILES_RS[@]}
if [[ $NUM_COMITTED -gt 0 ]]; then
    printf "Format %d files:\n" "${#COMMIT_FILES_RS[@]}"
    printf "  %s\n" "${COMMIT_FILES_RS[@]}"
    ( set -x; rustfmt --edition 2024 "${COMMIT_FILES_RS[@]}" )
    readarray -t FORMATTED_FILES < <( git --no-pager diff --name-only | grep ".rs$" )
    NUM_FORMATTED=${#FORMATTED_FILES[@]}
    if [[ $NUM_FORMATTED -gt 0 ]]; then
        printf "Formatted %d files\n" "$NUM_FORMATTED"
        ( set -x; git add "${COMMIT_FILES_RS[@]}" )
    else
        printf "No files formatted\n"
    fi
else
    printf "No .rs files in commit\n"
fi
