$ErrorActionPreference = 'Stop'
$env:DOTNET_CLI_TELEMETRY_OPTOUT = '1'
$projectRoot = $PSScriptRoot
$releaseDir = Join-Path $projectRoot 'dist'
dotnet run --project (Join-Path $projectRoot 'Monocc.Tests/Monocc.Tests.csproj') -c Release
if ($LASTEXITCODE -ne 0) { throw 'Core checks failed.' }
dotnet publish (Join-Path $projectRoot 'Monocc.Windows/Monocc.Windows.csproj') -c Release -r win-x64 --self-contained true -o $releaseDir
if ($LASTEXITCODE -ne 0) { throw 'Windows build failed.' }
$appPath = Join-Path $releaseDir 'MONOcc.exe'
$testProcess = Start-Process -FilePath $appPath -ArgumentList '--smoke-test' -Wait -PassThru
if ($testProcess.ExitCode -ne 0) { throw 'Windows interface checks failed.' }
Copy-Item $appPath (Join-Path $releaseDir 'MONOcc-0.2.0-Windows-x64-Setup.exe') -Force
Get-FileHash (Join-Path $releaseDir 'MONOcc-0.2.0-Windows-x64-Setup.exe') -Algorithm SHA256
