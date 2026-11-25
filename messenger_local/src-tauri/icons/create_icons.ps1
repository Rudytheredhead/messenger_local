Add-Type -AssemblyName System.Drawing

function New-RoundedRectPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $r * 2
  $path.AddArc($x, $y, $d, $d, 180, 90)
  $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
  $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
  $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
  $path.CloseFigure()
  return $path
}

function Render-TopIslandBitmap([int]$size) {
  # Rysujemy w 256x256 i skalujemy w dół z zachowaniem antyaliasingu dla ostrych mniejszych klatek
  $master = New-Object System.Drawing.Bitmap(256, 256, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $g = [System.Drawing.Graphics]::FromImage($master)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
  $g.Clear([System.Drawing.Color]::Transparent)

  # Tło zaokrąglonego kwadratu (Dark Glassmorphism)
  $bgPath = New-RoundedRectPath 10 10 236 236 58
  $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    [System.Drawing.PointF]::new(10, 10),
    [System.Drawing.PointF]::new(246, 246),
    [System.Drawing.Color]::FromArgb(255, 17, 20, 34),
    [System.Drawing.Color]::FromArgb(255, 29, 25, 58)
  )
  $g.FillPath($bgBrush, $bgPath)

  # Subtelna poświata wokół pigułki u góry
  $glowPath = New-Object System.Drawing.Drawing2D.GraphicsPath
  $glowPath.AddEllipse(18, 12, 220, 150)
  $glowBrush = New-Object System.Drawing.Drawing2D.PathGradientBrush($glowPath)
  $glowBrush.CenterColor = [System.Drawing.Color]::FromArgb(88, 99, 102, 241)
  $glowBrush.SurroundColors = @([System.Drawing.Color]::FromArgb(0, 56, 189, 248))
  $g.FillPath($glowBrush, $glowPath)

  # Szklany połysk u góry karty
  $g.SetClip($bgPath)
  $shineBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    [System.Drawing.PointF]::new(0, 10),
    [System.Drawing.PointF]::new(0, 130),
    [System.Drawing.Color]::FromArgb(38, 255, 255, 255),
    [System.Drawing.Color]::FromArgb(0, 255, 255, 255)
  )
  $g.FillRectangle($shineBrush, 10, 10, 236, 120)
  $g.ResetClip()

  # Zewnętrzna krawędź z gradientem cyan -> fiolet
  $rimBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    [System.Drawing.PointF]::new(10, 10),
    [System.Drawing.PointF]::new(246, 246),
    [System.Drawing.Color]::FromArgb(185, 56, 189, 248),
    [System.Drawing.Color]::FromArgb(195, 139, 92, 246)
  )
  $rimPen = New-Object System.Drawing.Pen($rimBrush, 4.5)
  $g.DrawPath($rimPen, $bgPath)

  # Główna górna kapsuła ("TopIsland")
  $pillPath = New-RoundedRectPath 40 54 176 64 32
  $pillBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    [System.Drawing.PointF]::new(40, 54),
    [System.Drawing.PointF]::new(216, 118),
    [System.Drawing.Color]::FromArgb(255, 56, 189, 248),
    [System.Drawing.Color]::FromArgb(255, 139, 92, 246)
  )
  $g.FillPath($pillBrush, $pillPath)

  # Szklana obwódka pigułki
  $pillRimPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(110, 255, 255, 255), 2.0)
  $g.DrawPath($pillRimPen, $pillPath)

  # Cztery świetlne punkty aplikacji wewnątrz pigułki
  $dotColors = @(
    [System.Drawing.Color]::FromArgb(245, 255, 255, 255),
    [System.Drawing.Color]::FromArgb(245, 214, 224, 255),
    [System.Drawing.Color]::FromArgb(245, 167, 243, 208),
    [System.Drawing.Color]::FromArgb(245, 237, 233, 254)
  )
  $dotXs = @(64, 98, 132, 166)
  for ($i = 0; $i -lt 4; $i++) {
    $dotBrush = New-Object System.Drawing.SolidBrush($dotColors[$i])
    $g.FillEllipse($dotBrush, $dotXs[$i], 74, 24, 24)
    $dotBrush.Dispose()
  }

  # Szewron rozwijania pod wyspą
  $chevPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(235, 236, 240, 255), 14.0)
  $chevPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $chevPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  $chevPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
  $chevPoints = @(
    [System.Drawing.PointF]::new(86, 152),
    [System.Drawing.PointF]::new(128, 190),
    [System.Drawing.PointF]::new(170, 152)
  )
  $g.DrawLines($chevPen, $chevPoints)

  $chevPen.Dispose()
  $pillRimPen.Dispose()
  $pillBrush.Dispose()
  $pillPath.Dispose()
  $rimPen.Dispose()
  $rimBrush.Dispose()
  $shineBrush.Dispose()
  $glowBrush.Dispose()
  $glowPath.Dispose()
  $bgBrush.Dispose()
  $bgPath.Dispose()
  $g.Dispose()

  if ($size -eq 256) {
    return $master
  }

  $scaled = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $sg = [System.Drawing.Graphics]::FromImage($scaled)
  $sg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $sg.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $sg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $sg.DrawImage($master, 0, 0, $size, $size)
  $sg.Dispose()
  $master.Dispose()
  return $scaled
}

function Save-PngIco([string]$outPath, [int[]]$sizes) {
  $frames = @()
  foreach ($sz in $sizes) {
    $bmp = Render-TopIslandBitmap $sz
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()
    $bmp.Dispose()
    $frames += ,@($sz, $bytes)
  }

  $fs = [System.IO.File]::Open($outPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
  $bw = New-Object System.IO.BinaryWriter($fs)
  # ICONDIR: Reserved (2), Type=1 (2), Count (2)
  $bw.Write([uint16]0)
  $bw.Write([uint16]1)
  $bw.Write([uint16]$frames.Count)

  $offset = 6 + (16 * $frames.Count)
  foreach ($f in $frames) {
    $sz = [int]$f[0]
    $data = [byte[]]$f[1]
    $dim = if ($sz -ge 256) { [byte]0 } else { [byte]$sz }
    $bw.Write($dim)                 # Width
    $bw.Write($dim)                 # Height
    $bw.Write([byte]0)              # ColorCount
    $bw.Write([byte]0)              # Reserved
    $bw.Write([uint16]1)            # Planes
    $bw.Write([uint16]32)           # BitCount
    $bw.Write([uint32]$data.Length) # BytesInRes
    $bw.Write([uint32]$offset)      # ImageOffset
    $offset += $data.Length
  }

  foreach ($f in $frames) {
    $bw.Write([byte[]]$f[1])
  }

  $bw.Close()
  $fs.Close()
}

$dir = $PSScriptRoot
$mainPng = Render-TopIslandBitmap 256
$mainPng.Save((Join-Path $dir "icon.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$mainPng.Dispose()

$trayPng = Render-TopIslandBitmap 32
$trayPng.Save((Join-Path $dir "tray-icon.png"), [System.Drawing.Imaging.ImageFormat]::Png)
$trayPng.Dispose()

Save-PngIco (Join-Path $dir "icon.ico") @(256, 64, 48, 32, 16)
Write-Host "TopIsland icons generated successfully."
