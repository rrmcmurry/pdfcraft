param(
    [Parameter(Mandatory=$true)][string[]]$Files,
    [Parameter(Mandatory=$true)][string]$SigningMetadataPath,
    [Parameter(Mandatory=$true)][string]$SigningDlibPath,
    [string]$ExpectedSigner='Robert McMurry'
)
$ErrorActionPreference='Stop'
$sdk=Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Directory | Where-Object {$_.Name -match '^10\.0\.\d+\.0$'} | Sort-Object Name -Descending | Select-Object -First 1
$signtool=Join-Path $sdk.FullName 'x64\signtool.exe'
foreach($path in $signtool,$SigningMetadataPath,$SigningDlibPath){if(!(Test-Path -LiteralPath $path)){throw "Required signing input missing: $path"}}
foreach($file in $Files){
    & $signtool sign /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib $SigningDlibPath /dmdf $SigningMetadataPath $file
    if($LASTEXITCODE){throw "Signing failed: $file"}
    $signature=Get-AuthenticodeSignature -LiteralPath $file
    if($signature.Status -ne 'Valid' -or !$signature.TimeStamperCertificate -or $signature.SignerCertificate.GetNameInfo('SimpleName',$false) -ne $ExpectedSigner){throw "Signature verification failed: $file"}
}
