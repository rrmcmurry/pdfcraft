param([string]$AppDirectory = (Join-Path $PSScriptRoot '..\..\..\dist\McMurryPDF'))
$ErrorActionPreference='Stop'
$app=[IO.Path]::GetFullPath($AppDirectory)
$dll=Join-Path $app 'McMurryPDFThumbnail.dll'
if(!(Test-Path $dll)){throw 'Stage the built provider DLL first.'}
$clsid='{8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}'
$iid='{E357FCCD-A995-4576-B01F-234630154E96}'
$classes='HKCU:\Software\Classes'
$choice='HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\UserChoice'
$before=Get-ItemProperty $choice
$backup=Join-Path $app 'association-backup'
New-Item -ItemType Directory -Force $backup | Out-Null
$records=foreach($base in '.pdf','Applications\McMurryPDF.exe','SystemFileAssociations\.pdf') {
    foreach($interface in $iid,'{BB2E617C-0920-11d1-9A0B-00C04FC2D6C1}') {
        $path="$classes\$base\shellex\$interface"
        [pscustomobject]@{Path=$path;Existed=(Test-Path $path);Value=if(Test-Path $path){(Get-Item $path).GetValue('')}else{$null}}
    }
}
$records | ConvertTo-Json | Set-Content (Join-Path $backup ('own-thumbnails-'+(Get-Date -Format 'yyyyMMdd-HHmmss')+'.json'))
New-Item "$classes\CLSID\$clsid\InprocServer32" -Force | Out-Null
Set-Item "$classes\CLSID\$clsid" -Value 'McMurryPDF Thumbnail Provider'
Set-Item "$classes\CLSID\$clsid\InprocServer32" -Value $dll
New-ItemProperty "$classes\CLSID\$clsid\InprocServer32" -Name ThreadingModel -Value Apartment -PropertyType String -Force | Out-Null
# No DisableProcessIsolation: Windows receives the document through IStream.
foreach($base in '.pdf','Applications\McMurryPDF.exe','SystemFileAssociations\.pdf') {
    $path="$classes\$base\shellex\$iid"
    New-Item $path -Force | Out-Null
    Set-Item $path -Value $clsid
}
$after=Get-ItemProperty $choice
if($before.ProgId -ne $after.ProgId -or $before.Hash -ne $after.Hash){throw 'Default viewer changed unexpectedly.'}
Add-Type 'using System;using System.Runtime.InteropServices;public static class OwnPdfThumbRefresh{[DllImport("shell32.dll")]public static extern void SHChangeNotify(uint e,uint f,IntPtr a,IntPtr b);}'
[OwnPdfThumbRefresh]::SHChangeNotify(0x08000000,0,[IntPtr]::Zero,[IntPtr]::Zero)
'Registered McMurryPDF provider; default preserved: '+$after.ProgId
