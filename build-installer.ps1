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

$targetDir = Join-Path $projectRoot 'work\cargo-target'
New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
$workRoot = (Resolve-Path (Join-Path $projectRoot 'work')).Path
Get-ChildItem -LiteralPath $workRoot -Directory | Where-Object { $_.Name -like 'release-*' -or $_.Name -like 'source-package-*' } | ForEach-Object {
    $orphanPath = (Resolve-Path -LiteralPath $_.FullName).Path
    if (-not $orphanPath.StartsWith($workRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Pasta de build fora do diretório temporário do projeto.'
    }
    Remove-Item -LiteralPath $orphanPath -Recurse -Force
}
$staging = Join-Path $workRoot ('release-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $staging | Out-Null
$previousTargetDir = $env:CARGO_TARGET_DIR
$env:CARGO_TARGET_DIR = $targetDir
Push-Location $projectRoot
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw 'npm ci falhou.' }
    npm run tauri build -- --no-bundle
    if ($LASTEXITCODE -ne 0) { throw 'A compilação Rust/Tauri falhou.' }
} finally {
    Pop-Location
    if ($null -eq $previousTargetDir) { Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue } else { $env:CARGO_TARGET_DIR = $previousTargetDir }
}

$compiledApp = Join-Path $targetDir 'release\cherry-browser.exe'
$webviewLoader = Join-Path $targetDir 'release\WebView2Loader.dll'
if (-not (Test-Path $webviewLoader)) {
    $webviewLoader = Get-ChildItem (Join-Path $targetDir 'release\build') -Filter 'WebView2Loader.dll' -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '[\\/]x64[\\/]WebView2Loader\.dll$' } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1 -ExpandProperty FullName
}
if (-not $webviewLoader -or -not (Test-Path $webviewLoader)) { throw 'WebView2Loader.dll não foi encontrada no build do WebView2.' }
$stageApp = Join-Path $staging 'Cherry-Browser.exe'
Copy-Item $compiledApp $stageApp -Force
$setupCode = Get-Content (Join-Path $projectRoot 'installer\Setup.cs') -Raw
$setupCode = $setupCode.Replace('__APP_VERSION__', $version).Replace('__RELEASE_CHANNEL__', $channel)
$generatedSource = Join-Path $staging 'Setup.generated.cs'
Set-Content -Path $generatedSource -Value $setupCode -Encoding UTF8
$stageSetup = Join-Path $staging 'Cherry-Browser-Setup.exe'
$icon = Join-Path $projectRoot 'src-tauri\icons\icon.ico'
& $csc /nologo /target:winexe /optimize+ /reference:System.Windows.Forms.dll /reference:System.Drawing.dll "/win32icon:$icon" "/resource:$stageApp,CherryBrowser.exe" "/resource:$webviewLoader,WebView2Loader.dll" "/out:$stageSetup" $generatedSource
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $stageSetup)) { throw 'A compilação do instalador gráfico falhou.' }

$portableDir = Join-Path $staging 'portable'
New-Item -ItemType Directory -Force -Path $portableDir | Out-Null
Copy-Item $stageApp (Join-Path $portableDir 'Cherry-Browser.exe') -Force
Copy-Item $webviewLoader (Join-Path $portableDir 'WebView2Loader.dll') -Force
$stagePortable = Join-Path $staging "Cherry-Browser-$version-$($manifest.channel)-portable.zip"
Compress-Archive -Path (Join-Path $portableDir '*') -DestinationPath $stagePortable -CompressionLevel Optimal

$sourceItems = @('src','src-tauri\capabilities','src-tauri\gen','src-tauri\icons','src-tauri\src','src-tauri\build.rs','src-tauri\Cargo.toml','src-tauri\Cargo.lock','src-tauri\tauri.conf.json','installer\Setup.cs','package.json','package-lock.json','vite.config.js','VERSION.json','README.md','CHANGELOG.md','build-installer.ps1') | ForEach-Object { Join-Path $projectRoot $_ }
$stageSource = Join-Path $staging 'Cherry-Browser-Fonte.zip'
Compress-Archive -Path $sourceItems -DestinationPath $stageSource -CompressionLevel Optimal

# Remove apenas builds de release antigos; a pasta do código-fonte permanece intacta.
$oldArtifacts = Get-ChildItem -LiteralPath $releaseDir -File | Where-Object {
    $_.Name -match '^(Cherry-Browser.*\.(exe|zip)|Leve-Browser.*\.(exe|zip)|~Leve-Browser-Setup\.CAB)$'
}
foreach ($artifact in $oldArtifacts) { Remove-Item -LiteralPath $artifact.FullName -Force }
Copy-Item $stageSetup (Join-Path $releaseDir 'Cherry-Browser-Setup.exe')
Copy-Item $stagePortable (Join-Path $releaseDir "Cherry-Browser-$version-$($manifest.channel)-portable.zip")
Copy-Item $stageSource (Join-Path $releaseDir 'Cherry-Browser-Fonte.zip')
$outputs = Get-Item (Join-Path $releaseDir 'Cherry-Browser-Setup.exe'),(Join-Path $releaseDir "Cherry-Browser-$version-$($manifest.channel)-portable.zip"),(Join-Path $releaseDir 'Cherry-Browser-Fonte.zip')
$stagingPath = (Resolve-Path -LiteralPath $staging).Path
if (-not $stagingPath.StartsWith($workRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Pasta de build fora do diretório temporário do projeto.'
}
Remove-Item -LiteralPath $stagingPath -Recurse -Force
$outputs | Select-Object Name, Length
