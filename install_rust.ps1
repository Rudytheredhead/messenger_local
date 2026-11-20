$url = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
$out = Join-Path $env:TEMP 'rustup-init.exe'
Invoke-WebRequest -Uri $url -OutFile $out
Start-Process -FilePath $out -ArgumentList '-y' -Wait