param([ValidateSet('Install','Thumbnails','Update','Ui','Uninstall','RemoveThumbnails')][string]$Phase='Install')
# Run only inside a disposable Windows Sandbox. The input directory must be read-only.
$ErrorActionPreference='Stop'
if(!(Test-Path C:\InstallInput) -or !(Test-Path C:\InstallTest)){throw 'Dedicated installer-test Sandbox required.'}
$work='C:\InstallTest'
$root=Join-Path $env:LOCALAPPDATA 'McMurrySoftware.McMurryPDF'
$app=Join-Path $root 'current\McMurryPDF.exe'
function Package([string]$Action){
    $out=Join-Path $work ($Action+'-out.json');$err=Join-Path $work ($Action+'-error.log')
    $p=Start-Process $app -ArgumentList @('--package-action',$Action) -WindowStyle Hidden -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
    # Start-Process -Wait waits for descendants too, including the GUI restarted by an update.
    $p.WaitForExit()
    if($p.ExitCode){throw "Package action $Action failed: $(Get-Content $out -Raw -ErrorAction SilentlyContinue)"}
    Get-Content $out -Raw | ConvertFrom-Json
}
try {
    switch($Phase){
        Install {
            $choiceKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\UserChoice'
            $before=(Get-ItemProperty $choiceKey -ErrorAction SilentlyContinue).ProgId
            $install=Start-Process 'C:\InstallInput\Probe-Setup.exe' -ArgumentList '--silent' -WindowStyle Hidden -Wait -PassThru
            if($install.ExitCode){throw "Setup failed: $($install.ExitCode)"}
            if(!(Test-Path $app)){throw 'GUI executable missing.'}
            $state=Package state
            if($state.current -ne '0.8.0'){throw "Unexpected probe version: $($state.current)"}
            $command=(Get-Item 'HKCU:\Software\Classes\Applications\McMurryPDF.exe\shell\open\command').GetValue('')
            if($command -ne ('"'+$app+'" "%1"')){throw 'File association is not using stable current path.'}
            $after=(Get-ItemProperty $choiceKey -ErrorAction SilentlyContinue).ProgId
            if($before -ne $after){throw 'Setup changed protected default choice.'}
            $cli=Join-Path $root 'current\McMurryPDF-cli.exe'
            $ErrorActionPreference='Continue'
            & $cli run --script C:\InstallInput\smoke.json --root $work 1> (Join-Path $work 'smoke-result.json') 2> (Join-Path $work 'smoke-errors.log')
            $status=$LASTEXITCODE;$ErrorActionPreference='Stop'
            if($status -or !(Test-Path (Join-Path $work 'baseline.pdf'))){throw 'Synthetic document creation failed.'}
            Set-Content (Join-Path $work 'app-path.txt') $app
            Set-Content (Join-Path $work 'install-passed.txt') 'Signed setup installed. Stable association registered; default preserved. CLI created synthetic PDF.'
        }
        Thumbnails {
            $app=(Get-Content (Join-Path $work 'app-path.txt') -Raw).Trim()
            $msi=Join-Path (Split-Path $app) 'McMurryPDF-Shell.msi'
            $p=Start-Process msiexec.exe -ArgumentList @('/i',('"'+$msi+'"'),'/qn','/norestart','/l*v',(Join-Path $work 'shell-msi.log')) -WindowStyle Hidden -Wait -PassThru
            if($p.ExitCode -notin 0,3010){throw "Thumbnail component failed: $($p.ExitCode)"}
            $provider=(Get-Item 'Registry::HKEY_CLASSES_ROOT\.pdf\shellex\{E357FCCD-A995-4576-B01F-234630154E96}').GetValue('')
            if($provider -ne '{8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}'){throw 'Wrong thumbnail provider.'}
            $adobe=@(Get-ChildItem HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall | Get-ItemProperty | Where-Object {$_.DisplayName -match 'Acrobat|Adobe Reader'})
            if($adobe.Count){throw 'Guest unexpectedly contains Acrobat.'}
            Copy-Item -LiteralPath $msi -Destination (Join-Path $work 'McMurryPDF-Shell.msi') -Force
            & C:\InstallInput\Test-Shell.ps1 -PdfPaths @((Join-Path $work 'baseline.pdf')) | ConvertTo-Json | Set-Content (Join-Path $work 'fresh-thumbnails.json')
            Set-Content (Join-Path $work 'thumbnails-passed.txt') 'Own provider generated a fresh thumbnail. Acrobat absent.'
        }
        Update {
            # Avoid the disabled virtual GPU; this guest tests packaging, not graphics drivers.
            $env:PDFCRAFT_RENDERER='gl'
            # An altered download must fail before any package is applied.
            $badFeed=Join-Path $work 'corrupt-feed'
            New-Item -ItemType Directory -Force $badFeed | Out-Null
            Copy-Item C:\InstallInput\feed\* $badFeed
            $package=Get-ChildItem $badFeed -Filter '*0.8.1*full.nupkg' | Select-Object -First 1
            if(!$package){throw 'Update test package missing.'}
            $stream=[IO.File]::Open($package.FullName,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite)
            try {$stream.Position=4096;$byte=$stream.ReadByte();$stream.Position=4096;$stream.WriteByte($byte -bxor 1)} finally {$stream.Dispose()}
            $env:MCMURRYPDF_UPDATE_SOURCE=$badFeed
            $rejected=$false
            try {Package download | Out-Null} catch {if($_ -match 'checksum|ChecksumInvalid'){$rejected=$true}else{throw}}
            if(!$rejected){throw 'Corrupt update package was accepted.'}
            $env:MCMURRYPDF_UPDATE_SOURCE='C:\InstallInput\feed'
            # These survive update and uninstall; keep real accounting files out of the guest.
            $settings=Join-Path $env:APPDATA 'McMurryPDF'
            New-Item -ItemType Directory -Force $settings | Out-Null
            Set-Content (Join-Path $settings 'installer-sentinel.txt') 'preserve settings'
            $check=Package check
            if($check.available -ne '0.8.1'){throw 'Published local update not found.'}
            $download=Package download
            if($download.phase -ne 'Ready'){throw 'Update not staged.'}
            $apply=Package apply
            if($apply.phase -ne 'Restarting'){throw 'Update not scheduled.'}
            for($i=0;$i -lt 60;$i++){
                Start-Sleep -Milliseconds 500
                $manifest=Join-Path $root 'current\sq.version'
                if((Test-Path $manifest) -and (Get-Content $manifest -Raw) -match '0\.8\.1'){break}
            }
            $state=Package state
            if($state.current -ne '0.8.1'){throw 'Installed version did not advance.'}
            if(!(Test-Path (Join-Path $settings 'installer-sentinel.txt')) -or !(Test-Path (Join-Path $work 'baseline.pdf'))){throw 'Update removed user data.'}
            Set-Content (Join-Path $work 'update-passed.txt') '0.8.0 -> 0.8.1 applied. Documents/settings preserved.'
        }
        Ui {
            foreach($p in Get-Process McMurryPDF -ErrorAction SilentlyContinue){if($p.Path -eq $app){$null=$p.CloseMainWindow();$null=$p.WaitForExit(10000)}}
            $env:PDFCRAFT_RENDERER='gl'
            $env:MCMURRYPDF_UPDATE_SOURCE='C:\InstallInput\feed'
            $control=Join-Path $work 'ui-control.json'
            Start-Process $app -ArgumentList @('--control',$control) -WindowStyle Hidden | Out-Null
            for($i=0;$i -lt 60;$i++){if(Test-Path $control){break};Start-Sleep -Milliseconds 500}
            $cli=Join-Path (Split-Path $app) 'McMurryPDF-cli.exe'
            & $cli ui --control $control command id=help.check_updates | Out-Null
            if($LASTEXITCODE){throw 'Installed UI update command failed.'}
            for($i=0;$i -lt 40;$i++){
                Start-Sleep -Milliseconds 500
                $inspect=& $cli ui --control $control inspect
                if($LASTEXITCODE){throw 'Installed UI inspect failed.'}
                if(($inspect -join '') -match 'You are up to date'){break}
            }
            if(($inspect -join '') -notmatch 'You are up to date'){throw 'Installed updates dialog did not report the current version.'}
            $inspect | Set-Content (Join-Path $work 'updates-ui.json')
            & $cli ui --control $control screenshot --out (Join-Path $work 'updates-ui.png') | Out-Null
            if($LASTEXITCODE){throw 'Installed UI screenshot failed.'}
            Set-Content (Join-Path $work 'ui-passed.txt') 'Signed installed GUI update dialog and control command verified.'
        }
        Uninstall {
            # Only close the empty test GUI launched by the update; no user documents were opened.
            foreach($p in Get-Process McMurryPDF -ErrorAction SilentlyContinue){if($p.Path -eq $app){$null=$p.CloseMainWindow();$null=$p.WaitForExit(10000)}}
            $p=Start-Process (Join-Path $root 'Update.exe') -ArgumentList @('uninstall','--silent') -WindowStyle Hidden -Wait -PassThru
            if($p.ExitCode){throw "Uninstall failed: $($p.ExitCode)"}
            if(Test-Path $app){throw 'Uninstall left installed GUI.'}
            if(!(Test-Path (Join-Path $work 'baseline.pdf')) -or !(Test-Path (Join-Path $env:APPDATA 'McMurryPDF\installer-sentinel.txt'))){throw 'Uninstall removed user data.'}
            Set-Content (Join-Path $work 'uninstall-passed.txt') 'GUI uninstalled; documents/settings preserved. Thumbnail component remains independently uninstallable.'
        }
        RemoveThumbnails {
            $p=Start-Process msiexec.exe -ArgumentList @('/x',(Join-Path $work 'McMurryPDF-Shell.msi'),'/qn','/norestart') -WindowStyle Hidden -Wait -PassThru
            if($p.ExitCode -notin 0,3010){throw "Thumbnail uninstall failed: $($p.ExitCode)"}
            if(Test-Path 'HKLM:\Software\Classes\CLSID\{8BE52C39-5E5E-4D32-A663-D1BEB26F65AD}'){throw 'Thumbnail class survived uninstall.'}
            if(!(Test-Path (Join-Path $work 'baseline.pdf'))){throw 'Thumbnail uninstall removed user document.'}
            Set-Content (Join-Path $work 'remove-thumbnails-passed.txt') 'Independent thumbnail component removed cleanly.'
        }
    }
    exit 0
} catch {
    $_ | Out-String | Set-Content (Join-Path $work ($Phase+'-failed.txt'))
    exit 1
}
