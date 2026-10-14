$ErrorActionPreference='Stop'
$vswhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$vs=& $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
Import-Module (Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64'
Push-Location $PSScriptRoot
try {
    & cl.exe /nologo /std:c++17 /O2 /EHsc /W4 /WX /MT /LD /DUNICODE /D_UNICODE thumbnail.cpp ole32.lib gdi32.lib advapi32.lib /link /DEF:thumbnail.def /OUT:McMurryPDFThumbnail.dll /DYNAMICBASE /NXCOMPAT /GUARD:CF
    if($LASTEXITCODE -ne 0){throw 'Thumbnail provider compilation failed.'}
} finally {Pop-Location}
