#!/bin/sh

set -e

cd $(dirname "$0")/./ashlang

for entry in test-vectors/*
do
  if echo $entry | grep "_test.ash" | grep -v "r1cs"
  then

    cargo run --release -- $(basename $entry | sed "s/.ash//") -i ./stdlib -i ./test-vectors -v 
  fi
done
