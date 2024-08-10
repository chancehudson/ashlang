#!/bin/sh

set -e

for entry in test-vectors/*
do
  if echo $entry | grep "_test.ash"
  then
    cargo run --release -- $entry -i ./stdlib -i ./test-vectors --asm
  fi
done
