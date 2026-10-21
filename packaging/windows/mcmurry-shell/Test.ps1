param([Parameter(Mandatory=$true)][string[]]$PdfPaths)
$ErrorActionPreference='Stop'
Add-Type -ReferencedAssemblies System.Drawing.Common,System.Drawing.Primitives,System.Private.Windows.GdiPlus,System.Private.Windows.Core @'
using System;using System.Runtime.InteropServices;using System.Runtime.InteropServices.ComTypes;using System.Drawing;
[ComImport,Guid("b824b49d-22ac-4161-ac8a-9916e8fa3f7f"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface InitStream{[PreserveSig]int Initialize(IStream stream,uint mode);}
[ComImport,Guid("e357fccd-a995-4576-b01f-234630154e96"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]interface Thumb{[PreserveSig]int GetThumbnail(uint size,out IntPtr bitmap,out uint alpha);}
public static class OwnThumbnailTest{
[DllImport("ole32.dll")]static extern int CoCreateInstance(ref Guid c,IntPtr outer,uint context,ref Guid iid,[MarshalAs(UnmanagedType.IUnknown)]out object obj);
[DllImport("shlwapi.dll",CharSet=CharSet.Unicode)]static extern int SHCreateStreamOnFileEx(string path,uint mode,uint attrs,bool create,IStream template,out IStream stream);
[DllImport("gdi32.dll")]static extern bool DeleteObject(IntPtr value);
public static string Run(string path,string output){Guid clsid=new Guid("8BE52C39-5E5E-4D32-A663-D1BEB26F65AD"),iid=new Guid("00000000-0000-0000-C000-000000000046");object obj;int hr=CoCreateInstance(ref clsid,IntPtr.Zero,1,ref iid,out obj);if(hr<0)Marshal.ThrowExceptionForHR(hr);IStream stream=null;IntPtr bmp=IntPtr.Zero;try{hr=SHCreateStreamOnFileEx(path,0x20,0,false,null,out stream);if(hr<0)Marshal.ThrowExceptionForHR(hr);hr=((InitStream)obj).Initialize(stream,0);if(hr<0)Marshal.ThrowExceptionForHR(hr);uint alpha;hr=((Thumb)obj).GetThumbnail(256,out bmp,out alpha);if(hr<0)return "0x"+hr.ToString("X8");using(var image=Image.FromHbitmap(bmp)){image.Save(output);return image.Width+"x"+image.Height+", alpha="+alpha;}}finally{if(bmp!=IntPtr.Zero)DeleteObject(bmp);if(stream!=null)Marshal.ReleaseComObject(stream);Marshal.ReleaseComObject(obj);}}
}
'@
$dir=Join-Path $env:TEMP 'McMurryPDF-render-diagnostic'
New-Item -ItemType Directory -Force $dir | Out-Null
$i=0
foreach($pdf in $PdfPaths){$i++; [pscustomobject]@{File=[IO.Path]::GetFileName($pdf);Thumbnail=[OwnThumbnailTest]::Run($pdf,(Join-Path $dir "own-thumbnail-$i.png"))}}
