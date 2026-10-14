param([string]$AppDirectory = (Join-Path $PSScriptRoot '..\..\..\dist\McMurryPDF'))
# Run in administrator PowerShell. Only our new COM class is registered machine-wide.
$ErrorActionPreference='Stop'
$identity=[Security.Principal.WindowsIdentity]::GetCurrent()
$principal=[Security.Principal.WindowsPrincipal]::new($identity)
if(!$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){throw 'Run this script in an administrator PowerShell.'}
$id='{8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}'
$app=[IO.Path]::GetFullPath($AppDirectory)
$dll=Join-Path $app 'McMurryPDFThumbnail.dll'
if(!(Test-Path $dll) -or !(Test-Path (Join-Path $app 'McMurryPDF-cli.exe'))){throw 'Provider DLL and McMurryPDF CLI must be staged first.'}
$classes='HKLM:\Software\Classes'
New-Item "$classes\CLSID\$id\InprocServer32" -Force | Out-Null
Set-Item "$classes\CLSID\$id" -Value 'McMurryPDF Thumbnail Provider'
Set-Item "$classes\CLSID\$id\InprocServer32" -Value $dll
New-ItemProperty "$classes\CLSID\$id\InprocServer32" -Name ThreadingModel -Value Apartment -PropertyType String -Force | Out-Null
New-ItemProperty "$classes\CLSID\$id" -Name AppID -Value $id -PropertyType String -Force | Out-Null
New-Item "$classes\AppID\$id" -Force | Out-Null
New-ItemProperty "$classes\AppID\$id" -Name DllSurrogate -Value '' -PropertyType String -Force | Out-Null
& (Join-Path $PSScriptRoot 'Register.ps1') -AppDirectory $app
# Remove just the earlier fallback installed by us, leaving Acrobat's own class alone.
foreach($base in '.pdf','Applications\McMurryPDF.exe','SystemFileAssociations\.pdf') {
    $key="HKCU:\Software\Classes\$base\shellex\{BB2E617C-0920-11d1-9A0B-00C04FC2D6C1}"
    if((Test-Path $key) -and (Get-Item $key).GetValue('') -eq '{F9DB5320-233E-11D1-9F84-707F02C10627}') {Remove-Item -LiteralPath $key -Force}
}
'Administrator COM registration complete. Run Test-Shell.ps1 to verify fresh thumbnails.'
