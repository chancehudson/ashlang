#!/bin/sh

set -e

for entry in test-vectors/*
do
  if echo $entry | grep "_test.ash" | grep -v "r1cs"
  then

    cargo run --features=prove --release -- -t tasm $(basename $entry | sed "s/.ash//") -i ./stdlib -i ./test-vectors -v -p 1 -s 1 -f foi
  fi
done
