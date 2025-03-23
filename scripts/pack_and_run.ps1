param(
    [switch]$Release
)

# Save original directory
$originalPath = $PWD

# Run the pack script with the Release parameter if provided
if ($Release) {
    & (Join-Path $PWD.Path "scripts\pack.ps1") -Release
} else {
    & (Join-Path $PWD.Path "scripts\pack.ps1")
}

# Remove old app package
Remove-AppPackage (Get-AppPackage -name 'ckaznable.gateofnotification').'PackageFullName'

# Add new app package
$msixPath = Join-Path $PWD.Path "out\GateOfNotification.msix"
Add-AppPackage -Path $msixPath

# Launch the app
Start-Process "shell:AppsFolder\$((Get-StartApps | Where-Object {$_.Name -eq 'Gate Of Notification'}).'AppID')"

# Restore original directory
Set-Location $originalPath