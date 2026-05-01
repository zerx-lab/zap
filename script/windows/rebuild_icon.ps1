param(
    [string]$Channel = 'oss'
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$repoRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSCommandPath))
$iconDir = Join-Path $repoRoot "app/channels/$Channel/icon/no-padding"
$icoPath = Join-Path $iconDir 'icon.ico'

if (-not (Test-Path $iconDir)) { throw "Icon dir not found: $iconDir" }

$plan = @(
    @{ S = 16;  F = '16x16.png'   },
    @{ S = 20;  F = $null         },
    @{ S = 24;  F = $null         },
    @{ S = 32;  F = '32x32.png'   },
    @{ S = 40;  F = $null         },
    @{ S = 48;  F = '48x48.png'   },
    @{ S = 64;  F = '64x64.png'   },
    @{ S = 128; F = '128x128.png' },
    @{ S = 256; F = '256x256.png' }
)

function Resize-PngTo {
    param([string]$SrcPath, [int]$Target)
    $src = [System.Drawing.Image]::FromFile($SrcPath)
    $bmp = New-Object System.Drawing.Bitmap $Target, $Target
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.DrawImage($src, 0, 0, $Target, $Target)
    $g.Dispose()
    $ms = New-Object IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()
    $bmp.Dispose()
    $src.Dispose()
    return ,$bytes
}

$entries = New-Object System.Collections.ArrayList
foreach ($p in $plan) {
    $size = [int]$p.S
    $file = $p.F
    $bytes = $null
    if ($file) {
        $path = Join-Path $iconDir $file
        if (Test-Path $path) {
            $bytes = [IO.File]::ReadAllBytes($path)
        }
    }
    if (-not $bytes) {
        $srcCandidate = Join-Path $iconDir '64x64.png'
        if (-not (Test-Path $srcCandidate)) { $srcCandidate = Join-Path $iconDir '128x128.png' }
        $bytes = Resize-PngTo -SrcPath $srcCandidate -Target $size
    }
    [void]$entries.Add(@{ Size = $size; Bytes = $bytes })
}

$count = $entries.Count
$headerSize = 6 + 16 * $count

$out = New-Object IO.MemoryStream
$bw = New-Object IO.BinaryWriter -ArgumentList $out

$bw.Write([UInt16]0)
$bw.Write([UInt16]1)
$bw.Write([UInt16]$count)

$offset = $headerSize
foreach ($e in $entries) {
    $sz = [int]$e.Size
    $w = $sz; if ($w -ge 256) { $w = 0 }
    $h = $sz; if ($h -ge 256) { $h = 0 }
    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0)
    $bw.Write([byte]0)
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]32)
    $bw.Write([UInt32]$e.Bytes.Length)
    $bw.Write([UInt32]$offset)
    $offset += $e.Bytes.Length
}

foreach ($e in $entries) {
    $bw.Write($e.Bytes)
}

$bw.Flush()
[IO.File]::WriteAllBytes($icoPath, $out.ToArray())
$bw.Dispose()
$out.Dispose()

$info = Get-Item $icoPath
$sizes = ($entries | ForEach-Object { $_.Size }) -join ','
Write-Host ("Wrote " + $icoPath + " (" + $info.Length + " bytes, " + $count + " sizes: " + $sizes + ")")
