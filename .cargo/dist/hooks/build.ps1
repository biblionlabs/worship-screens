$ErrorActionPreference = "Stop"

if ($env:DIST_TARGET -like "*windows-msvc*") {

  $gstVersion = "1.26.0"
  $gstFile = "gstreamer-1.0-msvc-x86_64-$gstVersion.msi"
  $gstUrl = "https://gstreamer.freedesktop.org/data/pkg/windows/1.26.0/msvc/$gstFile"

  Write-Host "Downloading GStreamer $gstVersion..."
  Invoke-WebRequest -Uri $gstUrl -OutFile $gstFile

  Write-Host "GStreamer MSI downloaded"
}
