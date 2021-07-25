#!/bin/bash

# Call example: bash build.sh 1.0.0

cargo build --release
cargo build --release --target x86_64-pc-windows-gnu
cp ./target/release/sampan deb/usr/bin/sampan

read -p "Did you update version number in deb/DEBIAN/control? [yY/*] " answer
while true
do
  case $answer in
    [yY]* ) dpkg -b deb/
            break;;
    * ) break;;
  esac
done

mkdir -p dist
mv deb.deb "dist/sampan-$1.deb"
cp ./target/release/sampan "dist/sampan-1.0.0"
cp ./target/release/sampan "dist/sampan-1.0.0"
