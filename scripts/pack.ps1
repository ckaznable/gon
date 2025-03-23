param(
    [switch]$Release
)

$originalPath = $PWD

# Determine configuration based on -Release switch
$configuration = if ($Release) { "release" } else { "debug" }

$targetExe = Join-Path $PWD.Path ".\target\$configuration\gon.exe" 
$windowsPackaging = Join-Path $PWD.Path ".\build\windows"
$outputDir = Join-Path $PWD.Path ".\out"
$outputMsix = Join-Path $outputDir "GateOfNotification.msix"
$certPath = Join-Path $PWD.Path ".\mycert.pfx"

# Show which configuration is being used
Write-Host "Packaging $configuration build..." -ForegroundColor Cyan

Copy-Item $targetExe $windowsPackaging

Set-Location $windowsPackaging

# Execute MakeAppx and check for errors
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\MakeAppx.exe" pack /d . /p $outputMsix /nv /o
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: MakeAppx failed with exit code $LASTEXITCODE" -ForegroundColor Red
    Set-Location $originalPath
    exit $LASTEXITCODE
}

Write-Host "MSIX package created successfully, now signing..." -ForegroundColor Green

# Execute SignTool and check for errors
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\SignTool.exe" sign /a /v /fd SHA256 /f $certPath /p qwertyuiop $outputMsix
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: SignTool signing failed with exit code $LASTEXITCODE" -ForegroundColor Red
    Set-Location $originalPath
    exit $LASTEXITCODE
}

Write-Host "Signing successful! MSIX package is ready." -ForegroundColor Green
Set-Location $originalPath
