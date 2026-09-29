<#
.SYNOPSIS
  Renders the Tetrachrome icon files from the SVG sources in assets/icon: PNGs and tetrachrome.ico.
.DESCRIPTION
  Sources: tetrachrome.svg (the full icon, 64 px and up) and tetrachrome-small.svg (wider gaps and a
  smaller corner radius, up to 48 px). Needs resvg on PATH (scoop install resvg). Running it again without
  changes to the SVGs gives the same files.
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$dir = Join-Path (Split-Path $PSScriptRoot -Parent) 'assets\icon'

function Render([string]$svg, [int]$size, [string]$png) {
    & resvg -w $size -h $size (Join-Path $dir $svg) (Join-Path $dir $png)
    if ($LASTEXITCODE -ne 0) { throw "resvg failed for $svg at $size px" }
}

foreach ($s in 16, 24, 32, 48) { Render 'tetrachrome-small.svg' $s "tetrachrome-$s.png" }
foreach ($s in 64, 128, 256, 512) { Render 'tetrachrome.svg' $s "tetrachrome-$s.png" }

# tetrachrome.ico: PNG frames (Windows Vista and later), 16 to 256.
$sizes = 16, 24, 32, 48, 64, 128, 256
$ms = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($ms)
$w.Write([uint16]0); $w.Write([uint16]1); $w.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
$blobs = @()
foreach ($s in $sizes) {
    $b = [System.IO.File]::ReadAllBytes((Join-Path $dir "tetrachrome-$s.png"))
    $blobs += , $b
    # Width and height bytes: 0 means 256.
    $w.Write([byte]($s % 256)); $w.Write([byte]($s % 256)); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([uint16]1); $w.Write([uint16]32); $w.Write([uint32]$b.Length); $w.Write([uint32]$offset)
    $offset += $b.Length
}
foreach ($b in $blobs) { $w.Write($b) }
$w.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $dir 'tetrachrome.ico'), $ms.ToArray())
Write-Host "Icon files written to $dir"
