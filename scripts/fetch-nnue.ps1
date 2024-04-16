$url = Invoke-RestMethod -Uri "https://api.github.com/repos/codedeliveryservice/RecklessNetworks/releases/latest" |
       Select-Object -ExpandProperty assets |
       Select-Object -First 1 -ExpandProperty browser_download_url

$directory = "..\networks"
if (-not (Test-Path $directory)) {
    New-Item -ItemType Directory -Path $directory | Out-Null
}

Invoke-WebRequest -Uri $url -OutFile "..\networks\model.nnue"
