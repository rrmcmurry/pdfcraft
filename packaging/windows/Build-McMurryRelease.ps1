param(
    [Parameter(Mandatory=$true)][string]$Version,
    [Parameter(Mandatory=$true)][string]$SigningMetadataPath,
    [Parameter(Mandatory=$true)][string]$SigningDlibPath,
    [Parameter(Mandatory=$true)][string]$ModelsDirectory,
    [ValidateSet('testing','stable')][string]$Channel='testing',
    [ValidatePattern('^[a-zA-Z0-9-]+$')][string]$ReleaseDirectoryName='',
    [string]$ShellVersion='',
    [switch]$SkipBuild,
    [switch]$UpdateProbe
)
$ErrorActionPreference='Stop'
$env:DOTNET_CLI_TELEMETRY_OPTOUT='1'
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if($Version -notmatch '^\d+\.\d+\.\d+$'){throw 'Version must be a three-part release version.'}
if(!$ShellVersion){$ShellVersion=$Version}
if($ShellVersion -notmatch '^\d+\.\d+\.\d+$'){throw 'ShellVersion must be a three-part MSI version.'}
if($Channel -eq 'stable' -and $UpdateProbe){throw 'Update probe packages cannot be stable releases.'}
if($Channel -eq 'stable'){
    if((git -C $root branch --show-current) -ne 'main' -or (git -C $root status --porcelain)){throw 'Stable releases require a clean main branch.'}
}
$releaseBase=Join-Path $root 'dist\mcmurry'
if(!$ReleaseDirectoryName){$ReleaseDirectoryName=$Channel}
$release=Join-Path $releaseBase $ReleaseDirectoryName
$stage=Join-Path $releaseBase "payload-$Version"
New-Item -ItemType Directory -Force $release,$stage | Out-Null
Push-Location $root
try {
    dotnet tool restore
    if($LASTEXITCODE){throw 'Tool restore failed.'}
    if(!$SkipBuild){
        cargo build --release --locked -p pdfcraft -p pdfcraft-cli -j 2
        if($LASTEXITCODE){throw 'Release compilation failed.'}
    }
    foreach($pair in @(@('pdfcraft.exe','McMurryPDF.exe'),@('pdfcraft-cli.exe','McMurryPDF-cli.exe'))){
        $source=Join-Path $root ('target\release\'+$pair[0])
        $info=(Get-Item $source).VersionInfo
        if($info.ProductName -ne 'McMurryPDF' -or $info.CompanyName -ne 'McMurry Software'){throw "Unbranded binary: $source"}
        if(!$UpdateProbe -and $info.ProductVersion -ne $Version){throw "Binary version $($info.ProductVersion) differs from package $Version"}
        Copy-Item -LiteralPath $source -Destination (Join-Path $stage $pair[1])
    }
    foreach($name in 'LICENSE-MIT','LICENSE-APACHE','NOTICE','ATTRIBUTION.md'){Copy-Item (Join-Path $root $name) $stage}
    foreach($name in 'LICENSE-velopack.txt','LICENSE-winreg.txt'){Copy-Item (Join-Path $PSScriptRoot $name) $stage}
    Copy-Item (Join-Path $root 'assets\mcmurry\mcmurrypdf.ico') $stage
    Copy-Item (Join-Path $root 'assets\mcmurry\LICENSE.txt') (Join-Path $stage 'LICENSE-logo.txt')
    Set-Content (Join-Path $stage 'ShellVersion.txt') -Value $ShellVersion -Encoding ascii
    if(!(Test-Path (Join-Path $ModelsDirectory 'ATTRIBUTION.txt'))){throw 'Verified OCR models and attribution are required.'}
    New-Item -ItemType Directory -Force (Join-Path $stage 'models') | Out-Null
    Get-ChildItem $ModelsDirectory -File | Where-Object {$_.Extension -ne '.part'} | Copy-Item -Destination (Join-Path $stage 'models')
    & (Join-Path $PSScriptRoot 'mcmurry-shell\Build.ps1')
    Copy-Item (Join-Path $PSScriptRoot 'mcmurry-shell\McMurryPDFThumbnail.dll') $stage
    $signer=Join-Path $PSScriptRoot 'Sign-McMurryArtifact.ps1'
    & $signer -Files @((Join-Path $stage 'McMurryPDF.exe'),(Join-Path $stage 'McMurryPDF-cli.exe'),(Join-Path $stage 'McMurryPDFThumbnail.dll')) -SigningMetadataPath $SigningMetadataPath -SigningDlibPath $SigningDlibPath
    $msi=Join-Path $stage 'McMurryPDF-Shell.msi'
    dotnet tool run wix -- build (Join-Path $PSScriptRoot 'mcmurry-shell\shell.wxs') -arch x64 -d "ShellVersion=$ShellVersion" -d "Payload=$stage" -o $msi
    if($LASTEXITCODE){throw 'Thumbnail MSI build failed.'}
    & $signer -Files @($msi) -SigningMetadataPath $SigningMetadataPath -SigningDlibPath $SigningDlibPath
    # Native DLL is installed by its MSI into Program Files, never loaded from Velopack/current.
    Remove-Item -LiteralPath (Join-Path $stage 'McMurryPDFThumbnail.dll')
    Get-ChildItem $stage -File -Filter '*.wixpdb' | Remove-Item
    dotnet tool run vpk -- pack --packId McMurrySoftware.McMurryPDF --packVersion $Version --packDir $stage --mainExe McMurryPDF.exe --packTitle McMurryPDF --packAuthors 'McMurry Software' --channel $Channel --runtime win-x64 --icon (Join-Path $stage 'mcmurrypdf.ico') --framework vcredist143-x64 --azureTrustedSignFile $SigningMetadataPath --outputDir $release --skip-updates
    if($LASTEXITCODE){throw 'Velopack packaging failed.'}
    $setup=Join-Path $release "McMurrySoftware.McMurryPDF-$Channel-Setup.exe"
    $signature=Get-AuthenticodeSignature $setup
    if($signature.Status -ne 'Valid' -or !$signature.TimeStamperCertificate -or $signature.SignerCertificate.GetNameInfo('SimpleName',$false) -ne 'Robert McMurry'){throw 'Installer signature verification failed.'}
    $verification=[IO.Path]::GetFullPath((Join-Path $releaseBase ('verify-'+[guid]::NewGuid())))
    if(!$verification.StartsWith(([IO.Path]::GetFullPath($releaseBase)+'\'),[StringComparison]::OrdinalIgnoreCase)){throw 'Unsafe verification output path.'}
    try {
        Expand-Archive -LiteralPath (Join-Path $release "McMurrySoftware.McMurryPDF-$Channel-Portable.zip") -DestinationPath $verification
        foreach($name in 'McMurryPDF.exe','McMurryPDF-cli.exe','McMurryPDF-Shell.msi'){
            $sig=Get-AuthenticodeSignature (Join-Path $verification "current\$name")
            if($sig.Status -ne 'Valid' -or !$sig.TimeStamperCertificate -or $sig.SignerCertificate.GetNameInfo('SimpleName',$false) -ne 'Robert McMurry'){throw "Packaged signature verification failed: $name"}
        }
        $updater=Get-AuthenticodeSignature (Join-Path $verification 'Update.exe')
        if($updater.Status -ne 'Valid'){throw 'Packaged updater signature verification failed.'}
    } finally {
        if(Test-Path -LiteralPath $verification){Remove-Item -LiteralPath $verification -Recurse -Force}
    }
    $files=Get-ChildItem $release -File | Where-Object {$_.Extension -in '.exe','.nupkg','.json' -or $_.Name -like 'RELEASES-*'} | ForEach-Object {[ordered]@{name=$_.Name;sha256=(Get-FileHash $_.FullName).Hash;length=$_.Length}}
    [ordered]@{product='McMurryPDF';version=$Version;channel=$Channel;shellVersion=$ShellVersion;updateProbe=[bool]$UpdateProbe;sourceCommit=(git rev-parse HEAD);sourceDirty=[bool](git status --porcelain);publisher='Robert McMurry';files=@($files)} | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $release "build-$Version.json")
    Write-Output "Verified signed installer: $setup"
} finally {Pop-Location}
