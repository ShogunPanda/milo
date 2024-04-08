#!/bin/sh

set -e

cd $(dirname $0)/..
rm -rf node_modules/@perseveranza-pets
ln -s ../../../parser/dist/wasm/CONFIGURATION/@perseveranza-pets node_modules/@perseveranza-pets
node src/reference.mjs