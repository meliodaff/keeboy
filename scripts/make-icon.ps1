# Generates assets/keeboy.ico (a keycap glyph) at 16/32/48 px, BMP-encoded.
# Run once after changing the design:
#   powershell -ExecutionPolicy Bypass -File scripts\make-icon.ps1
Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $root 'assets'
New-Item -ItemType Directory -Force -Path $assets | Out-Null
$outPath = Join-Path $assets 'keeboy.ico'

$sizes = @(16, 32, 48)

function New-RoundedPath([single]$x, [single]$y, [single]$w, [single]$h, [single]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

function New-Keycap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $inset = [single]([Math]::Max(1, $size / 16))
    $w = [single]($size - $inset * 2)
    $radius = [single]([Math]::Max(2, $size * 0.18))
    $path = New-RoundedPath $inset $inset $w $w $radius

    # Keycap face: top-lit gradient so it reads as a physical cap, light enough to
    # stay visible on a dark taskbar, with a dark border for light taskbars.
    $rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $rect,
        [System.Drawing.Color]::FromArgb(255, 245, 244, 242),
        [System.Drawing.Color]::FromArgb(255, 198, 194, 188),
        [System.Drawing.Drawing2D.LinearGradientMode]::Vertical)
    $g.FillPath($brush, $path)

    $penWidth = [single]([Math]::Max(1, $size / 14))
    $pen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 41, 37, 36), $penWidth)
    $g.DrawPath($pen, $path)

    # Glyph
    $fontSize = [single]($size * 0.52)
    $font = New-Object System.Drawing.Font('Segoe UI', $fontSize, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $fmt = New-Object System.Drawing.StringFormat
    $fmt.Alignment = [System.Drawing.StringAlignment]::Center
    $fmt.LineAlignment = [System.Drawing.StringAlignment]::Center
    $textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 28, 25, 23))
    $layout = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
    $g.DrawString('K', $font, $textBrush, $layout, $fmt)

    $g.Dispose()
    $brush.Dispose(); $pen.Dispose(); $font.Dispose(); $textBrush.Dispose(); $path.Dispose()
    return $bmp
}

function Get-Bgra([System.Drawing.Bitmap]$bmp) {
    $rect = New-Object System.Drawing.Rectangle(0, 0, $bmp.Width, $bmp.Height)
    $data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $bytes = New-Object byte[] ($data.Stride * $bmp.Height)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
    $bmp.UnlockBits($data)
    return @{ Bytes = $bytes; Stride = $data.Stride }
}

$images = @()
foreach ($size in $sizes) {
    $bmp = New-Keycap $size
    $px = Get-Bgra $bmp
    $stride = $px.Stride

    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($ms)

    # BITMAPINFOHEADER: height is doubled for the (unused) AND mask
    $bw.Write([uint32]40)
    $bw.Write([int32]$size)
    $bw.Write([int32]($size * 2))
    $bw.Write([uint16]1)
    $bw.Write([uint16]32)
    $bw.Write([uint32]0)
    $bw.Write([uint32]0)
    $bw.Write([int32]0); $bw.Write([int32]0)
    $bw.Write([uint32]0); $bw.Write([uint32]0)

    # BGRA rows, bottom-up
    for ($y = $size - 1; $y -ge 0; $y--) {
        $bw.Write($px.Bytes, $y * $stride, $size * 4)
    }

    # AND mask: 1bpp, rows padded to 4 bytes. Alpha already carries transparency.
    $maskRow = [Math]::Floor(($size + 31) / 32) * 4
    $bw.Write((New-Object byte[] ($maskRow * $size)))

    $bw.Flush()
    $images += @{ Size = $size; Data = $ms.ToArray() }
    $bw.Dispose(); $ms.Dispose(); $bmp.Dispose()
}

$out = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($out)
$w.Write([uint16]0)                  # reserved
$w.Write([uint16]1)                  # type: icon
$w.Write([uint16]$images.Count)

$offset = 6 + (16 * $images.Count)
foreach ($img in $images) {
    $w.Write([byte]$img.Size)
    $w.Write([byte]$img.Size)
    $w.Write([byte]0)                # palette colors
    $w.Write([byte]0)                # reserved
    $w.Write([uint16]1)              # planes
    $w.Write([uint16]32)             # bit depth
    $w.Write([uint32]$img.Data.Length)
    $w.Write([uint32]$offset)
    $offset += $img.Data.Length
}
foreach ($img in $images) { $w.Write($img.Data) }
$w.Flush()

[System.IO.File]::WriteAllBytes($outPath, $out.ToArray())
$w.Dispose(); $out.Dispose()

Write-Host "Wrote $outPath ($((Get-Item $outPath).Length) bytes, sizes: $($sizes -join ', '))"
