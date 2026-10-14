param([string]$AppDirectory = (Join-Path $PSScriptRoot '..\..\dist\McMurryPDF'), [string]$ModelsDirectory = (Join-Path $PSScriptRoot '..\..\assets\models'))
$ErrorActionPreference = 'Stop'
$source = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$stage = [IO.Path]::GetFullPath($AppDirectory)
# Preserve the stable Windows association: never publish with a new executable
# name or terminate running instances to replace it.
$stableExe = Join-Path $stage 'McMurryPDF.exe'
$running = Get-CimInstance Win32_Process -Filter "Name = 'McMurryPDF.exe'" | Where-Object { $_.ExecutablePath -eq $stableExe }
if ($running) { throw 'Close McMurryPDF windows before staging the update. The stable executable path will be preserved.' }
foreach ($binary in 'pdfcraft.exe','pdfcraft-cli.exe') {
    $file = Get-Item -LiteralPath (Join-Path $source "target\release\$binary")
    if ($file.VersionInfo.ProductName -ne 'McMurryPDF' -or $file.VersionInfo.CompanyName -ne 'McMurry Software') {
        throw "Refusing to stage an unbranded or stale executable: $binary"
    }
}
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item -LiteralPath (Join-Path $source 'target\release\pdfcraft.exe') -Destination (Join-Path $stage 'McMurryPDF.exe')
Copy-Item -LiteralPath (Join-Path $source 'target\release\pdfcraft-cli.exe') -Destination (Join-Path $stage 'McMurryPDF-cli.exe')
foreach ($name in 'LICENSE-MIT','LICENSE-APACHE','NOTICE','ATTRIBUTION.md') {
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination $stage
}
Copy-Item -LiteralPath (Join-Path $source 'assets\mcmurry\mcmurrypdf.ico') -Destination $stage
Copy-Item -LiteralPath (Join-Path $source 'assets\mcmurry\LICENSE.txt') -Destination (Join-Path $stage 'LICENSE-logo.txt')
$models = Join-Path $stage 'models'
New-Item -ItemType Directory -Force -Path $models | Out-Null
if (Test-Path -LiteralPath $ModelsDirectory) { Get-ChildItem -LiteralPath $ModelsDirectory -File | Copy-Item -Destination $models -Force }
# The office build uses its own per-user settings namespace, rather than the test guest's portable marker.
$metadata = foreach ($name in 'McMurryPDF.exe','McMurryPDF-cli.exe') {
    $path = Join-Path $stage $name
    [pscustomobject]@{Name=$name;SHA256=(Get-FileHash -LiteralPath $path).Hash;Product=(Get-Item -LiteralPath $path).VersionInfo.ProductName;Publisher=(Get-Item -LiteralPath $path).VersionInfo.CompanyName}
}
$metadata | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'build-metadata.json') -Encoding utf8
$metadata | Format-List
