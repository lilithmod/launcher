.\with-env.ps1 .\env\windows.env go build -o "./dist/lilith-launcher-windows-s1.exe" main.go
.\with-env.ps1 .\env\linux.env go build -o "./dist/lilith-launcher-linux-s1" main.go
.\with-env.ps1 .\env\macos.env go build -o "./dist/lilith-launcher-macos-s1" main.go
.\with-env.ps1 .\env\m1.env go build -o "./dist/lilith-launcher-m1-s1" main.go