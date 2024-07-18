#!/bin/sh

set -e

for entry in test-vectors/*
do
  cargo run -- $entry -i ./stdlib --asm
done
