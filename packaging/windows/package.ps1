# Windows installers from the portable zip: an MSI (WiX; Program Files, Start-menu
# shortcut, uninstall entry) and an MSIX with the reserved Microsoft Store identity.
# Both are signed, and test-installed, when ASCENSION_PFX_B64 / ASCENSION_PFX_PASSWORD
# are set; otherwise they are unsigned (Partner Center accepts an unsigned MSIX: the
# Store signs it). The public certificate is published beside them for sideloading.
param(
  [Parameter(Mandatory=$true)][ValidateSet('x64','arm64')][string]$Arch,
  [Parameter(Mandatory=$true)][string]$Version
)
$ErrorActionPreference = 'Stop'
$Target = if ($Arch -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Dist = Join-Path $Root 'dist'
$Work = Join-Path $Root 'app\target'
$Package = "ascension-$Version-$Target"
$Archive = Join-Path $Dist "$Package.zip"
$Unzip = Join-Path $Work "windows-unzip-$Arch"
if (Test-Path $Unzip) { Remove-Item $Unzip -Recurse -Force }
Expand-Archive $Archive -DestinationPath $Unzip
$Payload = Join-Path $Unzip $Package
if (-not (Test-Path (Join-Path $Payload 'rise-of-man.exe'))) { throw 'rise-of-man.exe missing from the portable payload' }
$Identity = Get-Content (Join-Path $PSScriptRoot 'store-identity.json') -Raw | ConvertFrom-Json

$Kit = 'C:\Program Files (x86)\Windows Kits\10\bin'
$HostDir = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64|x64' } else { 'x64' }
function Find-KitTool($Name) {
  $tool = Get-ChildItem $Kit -Recurse -Filter $Name | Where-Object { $_.FullName -match "\\($HostDir)\\" } |
    Sort-Object FullName -Descending | Select-Object -First 1
  if (-not $tool) { throw "Windows SDK $Name not found" }
  return $tool.FullName
}

# --- MSI -------------------------------------------------------------------------------
dotnet tool install --global wix --version 5.0.2
if ($LASTEXITCODE -ne 0) { throw 'WiX installation failed' }
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"
$MsiVersion = python (Join-Path $Root 'scripts\release\msix.py') --version $Version --output NUL --msi-version
if ($LASTEXITCODE -ne 0) { throw 'MSI version check failed' }
$Msi = Join-Path $Dist "ascension-$Version-windows-$Arch.msi"
wix build (Join-Path $PSScriptRoot 'ascension.wxs') -arch $Arch -d "Version=$MsiVersion" -d "Stage=$Payload" `
  -d "Icon=$PSScriptRoot\ascension.ico" -o $Msi
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Msi)) { throw 'MSI build failed' }
# WiX writes debug symbols beside the installer; they are not release files.
Get-ChildItem -Path $Dist -Filter '*.wixpdb' -File | Remove-Item -Force

# --- MSIX ------------------------------------------------------------------------------
$Stage = Join-Path $Work "msix-$Arch"
if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
Copy-Item $Payload $Stage -Recurse
$AssetsDir = Join-Path $Stage 'Assets'
New-Item $AssetsDir -ItemType Directory -Force | Out-Null
Add-Type -AssemblyName System.Drawing
$Icon = [System.Drawing.Image]::FromFile((Join-Path $Root 'packaging\ascension.png'))
foreach ($spec in @(@('StoreLogo',50),@('Square44x44Logo',44),@('Square150x150Logo',150))) {
  $bitmap = [System.Drawing.Bitmap]::new($spec[1],$spec[1])
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $graphics.Clear([System.Drawing.Color]::Transparent)
  $graphics.DrawImage($Icon,0,0,$spec[1],$spec[1])
  $bitmap.Save((Join-Path $AssetsDir "$($spec[0]).png"),[System.Drawing.Imaging.ImageFormat]::Png)
  $graphics.Dispose(); $bitmap.Dispose()
}
$Icon.Dispose()
python (Join-Path $Root 'scripts\release\msix.py') --version $Version --arch $Arch --output (Join-Path $Stage 'AppxManifest.xml')
if ($LASTEXITCODE -ne 0) { throw 'MSIX manifest generation failed' }
$MakeAppx = Find-KitTool 'makeappx.exe'
$Msix = Join-Path $Dist "ascension-$Version-windows-$Arch.msix"
& $MakeAppx pack /d $Stage /p $Msix /o
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Msix)) { throw 'makeappx pack failed (manifest or payload invalid)' }

# --- Signing and installation checks ---------------------------------------------------
$Artifacts = @($Msi, $Msix)
if (-not [string]::IsNullOrWhiteSpace($env:ASCENSION_PFX_B64) -and -not [string]::IsNullOrWhiteSpace($env:ASCENSION_PFX_PASSWORD)) {
  $Pfx = Join-Path $env:RUNNER_TEMP ("ascension-sign-" + [guid]::NewGuid().ToString('N') + '.pfx')
  try {
    [IO.File]::WriteAllBytes($Pfx, [Convert]::FromBase64String($env:ASCENSION_PFX_B64))
    $Certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new($Pfx, $env:ASCENSION_PFX_PASSWORD, [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::EphemeralKeySet)
    if (-not $Certificate.HasPrivateKey -or $Certificate.Subject -cne $Identity.publisher) { throw 'Signing certificate does not match the reserved Store publisher' }
    $SignTool = Find-KitTool 'signtool.exe'
    foreach ($file in @($Msi, $Msix)) {
      $signed = $false
      foreach ($attempt in 1..4) {
        & $SignTool sign /fd SHA256 /f $Pfx /p $env:ASCENSION_PFX_PASSWORD /tr http://timestamp.digicert.com /td SHA256 $file
        if ($LASTEXITCODE -eq 0) { $signed = $true; break }
        Start-Sleep -Seconds (15 * $attempt)
      }
      if (-not $signed) { throw "Signing $file failed" }
    }
    $Cer = Join-Path $Dist "ascension-$Version-windows-$Arch.cer"
    [IO.File]::WriteAllBytes($Cer, $Certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Cert))
    $Artifacts += $Cer
    # Trust the certificate on the runner only for the installation checks.
    $People = [Security.Cryptography.X509Certificates.X509Store]::new('TrustedPeople','LocalMachine')
    $People.Open('ReadWrite'); $People.Add($Certificate)
    try {
      Import-Module Appx -UseWindowsPowerShell -WarningAction SilentlyContinue -ErrorAction SilentlyContinue
      Add-AppxPackage -Path $Msix -ErrorAction Stop
      $Installed = Get-AppxPackage -Name $Identity.identityName
      if (-not $Installed -or $Installed.PackageFamilyName -cne $Identity.packageFamilyName) { throw 'Installed MSIX identity mismatch' }
      Remove-AppxPackage -Package $Installed.PackageFullName
      Write-Host "MSIX installed with family $($Installed.PackageFamilyName) and removed"
    } finally { $People.Remove($Certificate); $People.Close() }
  } finally { if (Test-Path $Pfx) { Remove-Item $Pfx -Force } }
} else {
  Write-Host "No ASCENSION_PFX_* secrets: $Msi and $Msix are unsigned"
}

# Install the MSI, check the program, shortcut and uninstall entry, then remove it.
$Log = Join-Path $Work "msi-install-$Arch.log"
$p = Start-Process msiexec.exe -ArgumentList @('/i', "`"$Msi`"", '/qn', '/norestart', '/l*v', "`"$Log`"") -Wait -PassThru
if ($p.ExitCode -ne 0) { Get-Content $Log -Tail 60; throw "MSI install failed with $($p.ExitCode)" }
$Exe = Join-Path $env:ProgramFiles 'Ascension\rise-of-man.exe'
if (-not (Test-Path $Exe)) { throw "MSI did not install $Exe" }
if (-not (Test-Path (Join-Path $env:ProgramFiles 'Ascension\rom.exe'))) { throw 'MSI did not install rom.exe' }
$Shortcut = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\Ascension\Ascension.lnk'
if (-not (Test-Path $Shortcut)) { throw "Start-menu shortcut missing: $Shortcut" }
$Uninstall = Get-ChildItem 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall' |
  ForEach-Object { Get-ItemProperty $_.PSPath } | Where-Object { $_.DisplayName -eq 'Ascension' }
if (-not $Uninstall) { throw 'No uninstall entry named Ascension' }
# rise-of-man.exe is a GUI-subsystem program: capture its output through a file, from another folder.
$OutFile = Join-Path $Work "version-$Arch.txt"
Start-Process -FilePath $Exe -ArgumentList '--version' -WorkingDirectory $env:TEMP -RedirectStandardOutput $OutFile -Wait -NoNewWindow
$Out = if (Test-Path $OutFile) { (Get-Content $OutFile -Raw).Trim() } else { '' }
if ($Out -ne "rise-of-man $Version") { throw "Installed rise-of-man.exe reports '$Out'" }
$p = Start-Process msiexec.exe -ArgumentList @('/x', "`"$Msi`"", '/qn', '/norestart') -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "MSI uninstall failed with $($p.ExitCode)" }
if (Test-Path $Exe) { throw 'MSI uninstall left the program behind' }
Write-Host "MSI installed to $Exe, version $Out, shortcut and uninstall entry present, removed"

foreach ($a in $Artifacts) {
  $digest = (Get-FileHash $a -Algorithm SHA256).Hash.ToLowerInvariant()
  "$digest  $(Split-Path $a -Leaf)" | Set-Content "$a.sha256"
}
