Add-Type -AssemblyName System.Drawing

$assetsDir = "D:\dev\my_word\assets"
if (!(Test-Path $assetsDir)) {
    New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null
}

function Create-WordIcon([int]$size) {
    $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.Clear([System.Drawing.Color]::Transparent)

    [float]$scale = $size / 256.0

    # 1. Background rounded tile
    [float]$pad = 12.0 * $scale
    [float]$tileW = $size - 2.0 * $pad
    [float]$tileH = $size - 2.0 * $pad
    [float]$radius = 42.0 * $scale
    [float]$d = $radius * 2.0

    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $path.AddArc($pad, $pad, $d, $d, [float]180.0, [float]90.0)
    $path.AddArc([float]($pad + $tileW - $d), $pad, $d, $d, [float]270.0, [float]90.0)
    $path.AddArc([float]($pad + $tileW - $d), [float]($pad + $tileH - $d), $d, $d, [float]0.0, [float]90.0)
    $path.AddArc($pad, [float]($pad + $tileH - $d), $d, $d, [float]90.0, [float]90.0)
    $path.CloseFigure()

    # Blue gradient fill
    $pt1 = [System.Drawing.PointF]::new($pad, $pad)
    $pt2 = [System.Drawing.PointF]::new([float]($pad + $tileW), [float]($pad + $tileH))
    $cTop = [System.Drawing.Color]::FromArgb(255, 30, 90, 214)     # #1e5ad6
    $cBottom = [System.Drawing.Color]::FromArgb(255, 12, 38, 105)  # #0c2669
    $gradBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new($pt1, $pt2, $cTop, $cBottom)
    $g.FillPath($gradBrush, $path)

    # Subtle inner border
    $borderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(80, 255, 255, 255), [float](2.0 * $scale))
    $g.DrawPath($borderPen, $path)

    # 2. Document page graphic (white sheet with folded corner)
    [float]$docW = 110.0 * $scale
    [float]$docH = 140.0 * $scale
    [float]$docX = 100.0 * $scale
    [float]$docY = 58.0 * $scale
    [float]$fold = 28.0 * $scale

    $docPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $docPath.AddLine($docX, $docY, [float]($docX + $docW - $fold), $docY)
    $docPath.AddLine([float]($docX + $docW), [float]($docY + $fold), [float]($docX + $docW), [float]($docY + $docH))
    $docPath.AddLine($docX, [float]($docY + $docH), $docX, $docY)
    $docPath.CloseFigure()

    $docBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(245, 248, 255))
    $g.FillPath($docBrush, $docPath)

    # Fold corner fill
    $foldPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $foldPath.AddLine([float]($docX + $docW - $fold), $docY, [float]($docX + $docW), [float]($docY + $fold))
    $foldPath.AddLine([float]($docX + $docW - $fold), [float]($docY + $fold), [float]($docX + $docW - $fold), $docY)
    $foldPath.CloseFigure()
    $foldBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(200, 215, 235))
    $g.FillPath($foldBrush, $foldPath)

    # Text lines on document sheet
    $lineBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(180, 100, 140, 210))
    $linePen = [System.Drawing.Pen]::new($lineBrush, [float](5.0 * $scale))
    $linePen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $linePen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

    $g.DrawLine($linePen, [float]($docX + 24.0 * $scale), [float]($docY + 48.0 * $scale), [float]($docX + $docW - 18.0 * $scale), [float]($docY + 48.0 * $scale))
    $g.DrawLine($linePen, [float]($docX + 24.0 * $scale), [float]($docY + 68.0 * $scale), [float]($docX + $docW - 18.0 * $scale), [float]($docY + 68.0 * $scale))
    $g.DrawLine($linePen, [float]($docX + 24.0 * $scale), [float]($docY + 88.0 * $scale), [float]($docX + $docW - 32.0 * $scale), [float]($docY + 88.0 * $scale))
    $g.DrawLine($linePen, [float]($docX + 24.0 * $scale), [float]($docY + 108.0 * $scale), [float]($docX + $docW - 44.0 * $scale), [float]($docY + 108.0 * $scale))

    # 3. Stylized 'W' Badge on left side
    [float]$badgeW = 92.0 * $scale
    [float]$badgeH = 104.0 * $scale
    [float]$badgeX = 38.0 * $scale
    [float]$badgeY = 76.0 * $scale
    [float]$badgeRadius = 16.0 * $scale
    [float]$bd = $badgeRadius * 2.0

    $bPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $bPath.AddArc($badgeX, $badgeY, $bd, $bd, [float]180.0, [float]90.0)
    $bPath.AddArc([float]($badgeX + $badgeW - $bd), $badgeY, $bd, $bd, [float]270.0, [float]90.0)
    $bPath.AddArc([float]($badgeX + $badgeW - $bd), [float]($badgeY + $badgeH - $bd), $bd, $bd, [float]0.0, [float]90.0)
    $bPath.AddArc($badgeX, [float]($badgeY + $badgeH - $bd), $bd, $bd, [float]90.0, [float]90.0)
    $bPath.CloseFigure()

    # Badge gradient
    $bPt1 = [System.Drawing.PointF]::new($badgeX, $badgeY)
    $bPt2 = [System.Drawing.PointF]::new($badgeX, [float]($badgeY + $badgeH))
    $bTop = [System.Drawing.Color]::FromArgb(255, 37, 99, 235)   # #2563eb
    $bBottom = [System.Drawing.Color]::FromArgb(255, 29, 78, 216) # #1d4ed8
    $bGrad = [System.Drawing.Drawing2D.LinearGradientBrush]::new($bPt1, $bPt2, $bTop, $bBottom)
    $g.FillPath($bGrad, $bPath)

    $bBorder = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(160, 255, 255, 255), [float](2.0 * $scale))
    $g.DrawPath($bBorder, $bPath)

    # Draw Letter 'W'
    $wPath = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $wFontFamily = [System.Drawing.FontFamily]::GenericSansSerif
    [float]$wFontSize = 58.0 * $scale
    $wFormat = [System.Drawing.StringFormat]::new()
    $wFormat.Alignment = [System.Drawing.StringAlignment]::Center
    $wFormat.LineAlignment = [System.Drawing.StringAlignment]::Center

    $wRect = [System.Drawing.RectangleF]::new($badgeX, [float]($badgeY + 2.0 * $scale), $badgeW, $badgeH)
    $wPath.AddString("W", $wFontFamily, [int][System.Drawing.FontStyle]::Bold, $wFontSize, $wRect, $wFormat)

    $wBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $g.FillPath($wBrush, $wPath)

    $g.Dispose()
    return $bmp
}

# Function to save multi-resolution ICO file
function Save-MultiResIco([System.Drawing.Bitmap[]]$bitmaps, [string]$icoPath) {
    $fs = [System.IO.File]::Create($icoPath)
    $bw = [System.IO.BinaryWriter]::new($fs)

    # ICONDIR
    $bw.Write([UInt16]0) # Reserved
    $bw.Write([UInt16]1) # Resource Type (1 for icon)
    $bw.Write([UInt16]$bitmaps.Length) # Image count

    # Memory streams for PNG encoded bytes of each image
    $streams = @()
    foreach ($b in $bitmaps) {
        $ms = [System.IO.MemoryStream]::new()
        $b.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $streams += $ms
    }

    $offset = 6 + ($bitmaps.Length * 16)

    # ICONDIRENTRY for each
    for ($i = 0; $i -lt $bitmaps.Length; $i++) {
        $b = $bitmaps[$i]
        $ms = $streams[$i]
        $w = if ($b.Width -ge 256) { [byte]0 } else { [byte]$b.Width }
        $h = if ($b.Height -ge 256) { [byte]0 } else { [byte]$b.Height }

        $bw.Write($w) # Width
        $bw.Write($h) # Height
        $bw.Write([byte]0) # Colors (0 = no palette)
        $bw.Write([byte]0) # Reserved
        $bw.Write([UInt16]1) # Color planes
        $bw.Write([UInt16]32) # Bits per pixel
        $bw.Write([UInt32]$ms.Length) # Size of image data
        $bw.Write([UInt32]$offset) # Offset
        $offset += $ms.Length
    }

    # Write image data
    foreach ($ms in $streams) {
        $bytes = $ms.ToArray()
        $bw.Write($bytes)
        $ms.Dispose()
    }

    $bw.Flush()
    $bw.Close()
    $fs.Close()
}

$bmp256 = Create-WordIcon 256
$bmp64  = Create-WordIcon 64
$bmp48  = Create-WordIcon 48
$bmp32  = Create-WordIcon 32
$bmp16  = Create-WordIcon 16

$bmp256.Save("$assetsDir\icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
Save-MultiResIco @($bmp256, $bmp64, $bmp48, $bmp32, $bmp16) "$assetsDir\icon.ico"

Write-Output "Successfully created $assetsDir\icon.png and $assetsDir\icon.ico"
