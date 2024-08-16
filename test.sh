#!/bin/sh

set -e

for entry in test-vectors/*
do
  if echo $entry | grep "_test.ash"
  then

    cargo run --release -- $(basename $entry | sed "s/.ash//") -i ./stdlib -i ./test-vectors --asm -p 1 -s 1
  fi
done
