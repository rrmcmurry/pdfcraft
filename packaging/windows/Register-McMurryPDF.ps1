param([string]$AppDirectory = (Join-Path $PSScriptRoot '..\..\dist\McMurryPDF'))
$ErrorActionPreference = 'Stop'
$appDir = [IO.Path]::GetFullPath($AppDirectory)
$exe = Join-Path $appDir 'McMurryPDF.exe'
if (!(Test-Path -LiteralPath $exe)) { throw 'Stage McMurryPDF.exe before registering it.' }
$command = '"' + $exe + '" "%1"'
$icon = '"' + (Join-Path $appDir 'mcmurrypdf.ico') + '"'
$classes = 'HKCU:\Software\Classes'
# Keep existing Windows UserChoice intact. Redirect our older application IDs
# so the currently selected default continues to work through the stable path.
$ids = @('Applications\McMurryPDF.exe')
$currentChoice = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\UserChoice' -ErrorAction SilentlyContinue).ProgId
$applications = Join-Path $classes 'Applications'
if (Test-Path $applications) {
    foreach ($old in @(Get-ChildItem $applications | Where-Object { $_.PSChildName -like 'McMurryPDF-*.exe' })) {
        $oldId = 'Applications\' + $old.PSChildName
        if ($currentChoice -eq $oldId) { $ids += $oldId }
        else { Remove-Item -LiteralPath $old.PSPath -Recurse -Force }
    }
}
$documentKey = Join-Path $classes 'McMurryPDF.Document'
if ($currentChoice -eq 'McMurryPDF.Document') { $ids += 'McMurryPDF.Document' }
elseif (Test-Path $documentKey) { Remove-Item -LiteralPath $documentKey -Recurse -Force }
foreach ($id in $ids) {
    $base = Join-Path $classes $id
    New-Item "$base\shell\open\command" -Force | Out-Null
    Set-Item "$base\shell\open\command" -Value $command
    New-ItemProperty $base -Name FriendlyAppName -Value 'McMurryPDF' -PropertyType String -Force | Out-Null
    New-Item "$base\DefaultIcon" -Force | Out-Null
    Set-Item "$base\DefaultIcon" -Value $icon
    if ($id -like 'Applications\McMurryPDF-*.exe') {
        New-ItemProperty $base -Name NoOpenWith -Value '' -PropertyType String -Force | Out-Null
    }
}
New-Item "$classes\Applications\McMurryPDF.exe\SupportedTypes" -Force | Out-Null
New-ItemProperty "$classes\Applications\McMurryPDF.exe\SupportedTypes" -Name '.pdf' -Value '' -PropertyType String -Force | Out-Null
New-Item "$classes\.pdf\OpenWithProgids" -Force | Out-Null
Remove-ItemProperty "$classes\.pdf\OpenWithProgids" -Name 'McMurryPDF.Document' -ErrorAction SilentlyContinue
Remove-ItemProperty "$classes\.pdf\OpenWithProgids" -Name 'Applications\McMurryPDF.exe' -ErrorAction SilentlyContinue
# Remove only obsolete McMurryPDF names from Explorer's recent-app list.
$recent = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\OpenWithList'
if (Test-Path $recent) {
    $key = Get-Item $recent
    $mru = [string]$key.GetValue('MRUList')
    foreach ($name in $key.GetValueNames()) {
        if ($name -ne 'MRUList' -and [string]$key.GetValue($name) -like 'McMurryPDF-*.exe') {
            Remove-ItemProperty $recent -Name $name
            $mru = $mru.Replace($name, '')
        }
    }
    New-ItemProperty $recent -Name MRUList -Value $mru -PropertyType String -Force | Out-Null
}
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'McMurryPDF.lnk'))
$shortcut.TargetPath = $exe
$shortcut.WorkingDirectory = $appDir
$shortcut.IconLocation = Join-Path $appDir 'mcmurrypdf.ico'
$shortcut.Save()
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class McMurryAssociationRefresh { [DllImport("shell32.dll")] public static extern void SHChangeNotify(uint e, uint f, IntPtr a, IntPtr b); }'
[McMurryAssociationRefresh]::SHChangeNotify(0x08000000,0,[IntPtr]::Zero,[IntPtr]::Zero)
Write-Output "Registered stable executable: $exe"
