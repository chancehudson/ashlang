#!/bin/sh

set -e

for entry in test-vectors/*
do
  if echo $entry | grep "_test.ash"
  then

    cargo run --release -- -t tasm $(basename $entry | sed "s/.ash//") -i ./stdlib -i ./test-vectors -v -p 1 -s 1
  fi
done
