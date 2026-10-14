param([Parameter(Mandatory=$true)][string[]]$PdfPaths)
$ErrorActionPreference='Stop'
Add-Type @'
using System;using System.Runtime.InteropServices;
[StructLayout(LayoutKind.Sequential)]public struct Size{public int cx,cy;}
[ComImport,Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface ImageFactory{[PreserveSig]int GetImage(Size s,uint flags,out IntPtr bitmap);}
public static class ShellThumbTest{
[DllImport("shell32.dll",CharSet=CharSet.Unicode)]static extern int SHCreateItemFromParsingName(string p,IntPtr b,ref Guid iid,out ImageFactory obj);
[DllImport("gdi32.dll")]static extern bool DeleteObject(IntPtr o);
public static string Run(string p){Guid iid=new Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b");ImageFactory obj;int hr=SHCreateItemFromParsingName(p,IntPtr.Zero,ref iid,out obj);if(hr<0)Marshal.ThrowExceptionForHR(hr);IntPtr bmp=IntPtr.Zero;try{hr=obj.GetImage(new Size{cx=256,cy=256},8,out bmp);if(hr<0)Marshal.ThrowExceptionForHR(hr);if(bmp==IntPtr.Zero)throw new Exception("No bitmap returned");return "Actual thumbnail generated (icon fallback disabled)";}finally{if(bmp!=IntPtr.Zero)DeleteObject(bmp);Marshal.ReleaseComObject(obj);}}
}
'@
$dir=Join-Path $env:TEMP ('McMurryPDF-fresh-thumbnails-'+[guid]::NewGuid())
New-Item -ItemType Directory $dir | Out-Null
foreach($pdf in $PdfPaths){
    $copy=Join-Path $dir ([IO.Path]::GetFileName($pdf))
    Copy-Item -LiteralPath $pdf -Destination $copy
    [pscustomobject]@{File=[IO.Path]::GetFileName($pdf);Result=[ShellThumbTest]::Run($copy)}
}
