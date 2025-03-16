# Save original directory
$originalPath = $PWD

# Generate a more robust self-signed certificate with simplified parameters
$cert = New-SelfSignedCertificate -Type Custom -Subject "CN=ckaznable" `
    -KeyUsage DigitalSignature `
    -FriendlyName "ckaznable Certificate" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3", "2.5.29.19={text}") `
    -KeyLength 2048 `
    -KeyAlgorithm RSA `
    -HashAlgorithm SHA256 `
    -NotAfter (Get-Date).AddYears(5)

# Display certificate information
$cert | Format-List Subject, Thumbprint, FriendlyName

# Set password for certificate export
$password = ConvertTo-SecureString -String "qwertyuiop" -Force -AsPlainText

# Get certificate path
$certPath = "Cert:\CurrentUser\My\" + $cert.Thumbprint

# Set PFX output path
$pfxPath = Join-Path $PWD.Path "mycert.pfx"

# Export as PFX file
Export-PfxCertificate -Cert $certPath -FilePath $pfxPath -Password $password

# Install certificate to trusted root store (requires admin privileges)
Write-Host "Do you want to install this certificate to the trusted root store? (Y/N)" -ForegroundColor Yellow
$response = Read-Host
if ($response -eq 'Y' -or $response -eq 'y') {
    try {
        Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\LocalMachine\Root -Password $password
        Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation Cert:\LocalMachine\TrustedPublisher -Password $password
        Write-Host "Certificate installed successfully to trusted stores" -ForegroundColor Green
    } catch {
        Write-Host "Failed to install certificate. Make sure you're running as Administrator." -ForegroundColor Red
        Write-Host $_.Exception.Message
    }
}

Write-Host "Certificate generated at: $pfxPath" -ForegroundColor Green

# Restore original directory
Set-Location $originalPath