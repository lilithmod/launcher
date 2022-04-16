#!/bin/bash

./with-env.sh ./env/windows.env go build -o "./dist/lilith-launcher-windows-s1.exe" main.go
./with-env.sh ./env/linux.env go build -o "./dist/lilith-launcher-linux-s1" main.go
./with-env.sh ./env/macos.env go build -o "./dist/lilith-launcher-macos-s1" main.go
./with-env.sh ./env/m1.env go build -o "./dist/lilith-launcher-m1-s1" main.go