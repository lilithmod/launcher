.\with-env.ps1 .\env\windows.env go build -o "./dist/lilith-launcher-windows-s2.exe" -ldflags "-s" main.go
.\with-env.ps1 .\env\linux.env go build -o "./dist/lilith-launcher-linux-s2" -ldflags "-s" main.go
.\with-env.ps1 .\env\macos.env go build -o "./dist/lilith-launcher-macos-s2" -ldflags "-s" main.go
.\with-env.ps1 .\env\m1.env go build -o "./dist/lilith-launcher-m1-s2" -ldflags "-s" main.go