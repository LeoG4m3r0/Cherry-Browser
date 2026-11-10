$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$releaseDir = Split-Path -Parent $projectRoot
$manifest = Get-Content (Join-Path $projectRoot 'VERSION.json') -Raw | ConvertFrom-Json
$config = Get-Content (Join-Path $projectRoot 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$package = Get-Content (Join-Path $projectRoot 'package.json') -Raw | ConvertFrom-Json
$cargoText = Get-Content (Join-Path $projectRoot 'src-tauri\Cargo.toml') -Raw
if ($cargoText -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw 'Não encontrei a versão do Cargo.toml.' }
$cargoVersion = $Matches[1]
if ($manifest.version -ne $config.version -or $manifest.version -ne $package.version -or $manifest.version -ne $cargoVersion) {
    throw 'As versões de VERSION.json, tauri.conf.json, package.json e Cargo.toml precisam ser iguais.'
}
$version = $manifest.version
$channel = $manifest.channel.ToUpperInvariant()
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
if (-not (Test-Path $csc)) { $csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework\v4.0.30319\csc.exe' }
if (-not (Test-Path $csc)) { throw 'O compilador C# do Windows não está disponível.' }

$staging = Join-Path $projectRoot ('work\release-' + [Guid]::NewGuid().ToString('N'))
$targetDir = Join-Path $staging 'cargo-target'
New-Item -ItemType Directory -Force -Path $staging | Out-Null
$previousTargetDir = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = $targetDir
Push-Location $projectRoot
try {
    npm install
    if ($LASTEXITCODE -ne 0) { throw 'npm install falhou.' }
    npm run tauri build -- --no-bundle
    if ($LASTEXITCODE -ne 0) { throw 'A compilação Rust/Tauri falhou.' }
} finally {
    Pop-Location
    if ($null -eq $previousTargetDir) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $previousTargetDir }
}

$compiledApp = Join-Path $targetDir 'release\cherry-browser.exe'
$stageApp = Join-Path $staging 'Cherry-Browser.exe'
Copy-Item $compiledApp $stageApp -Force
$setupCode = Get-Content (Join-Path $projectRoot 'installer\Setup.cs') -Raw
$setupCode = $setupCode.Replace('__APP_VERSION__', $version).Replace('__RELEASE_CHANNEL__', $channel)
$generatedSource = Join-Path $staging 'Setup.generated.cs'
Set-Content -Path $generatedSource -Value $setupCode -Encoding UTF8
$stageSetup = Join-Path $staging 'Cherry-Browser-Setup.exe'
$icon = Join-Path $projectRoot 'src-tauri\icons\icon.ico'
& $csc /nologo /target:winexe /optimize+ /reference:System.Windows.Forms.dll /reference:System.Drawing.dll "/win32icon:$icon" "/resource:$stageApp,CherryBrowser.exe" "/out:$stageSetup" $generatedSource
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $stageSetup)) { throw 'A compilação do instalador gráfico falhou.' }

$sourceItems = @('src','src-tauri\capabilities','src-tauri\gen','src-tauri\icons','src-tauri\src','src-tauri\build.rs','src-tauri\Cargo.toml','src-tauri\Cargo.lock','src-tauri\tauri.conf.json','installer\Setup.cs','package.json','package-lock.json','vite.config.js','VERSION.json','README.md','CHANGELOG.md','build-installer.ps1') | ForEach-Object { Join-Path $projectRoot $_ }
$stageSource = Join-Path $staging 'Cherry-Browser-Fonte.zip'
Compress-Archive -Path $sourceItems -DestinationPath $stageSource -CompressionLevel Optimal

# Remove apenas builds de release antigos; a pasta do código-fonte permanece intacta.
$oldArtifacts = Get-ChildItem -LiteralPath $releaseDir -File | Where-Object {
    $_.Name -match '^(Cherry-Browser.*\.(exe|zip)|Leve-Browser.*\.(exe|zip)|~Leve-Browser-Setup\.CAB)$'
}
foreach ($artifact in $oldArtifacts) { Remove-Item -LiteralPath $artifact.FullName -Force }
Copy-Item $stageApp (Join-Path $releaseDir ("Cherry-Browser-$version-$($manifest.channel).exe"))
Copy-Item $stageSetup (Join-Path $releaseDir 'Cherry-Browser-Setup.exe')
Copy-Item $stageSource (Join-Path $releaseDir 'Cherry-Browser-Fonte.zip')
Get-Item (Join-Path $releaseDir 'Cherry-Browser-Setup.exe'),(Join-Path $releaseDir ("Cherry-Browser-$version-$($manifest.channel).exe")),(Join-Path $releaseDir 'Cherry-Browser-Fonte.zip') | Select-Object Name, Length
