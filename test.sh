#!/bin/sh

set -e

for entry in test-vectors/*
do
  cargo run -- $entry -i ./stdlib -i ./test-vectors --asm
done
