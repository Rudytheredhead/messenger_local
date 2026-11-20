Add-Type -AssemblyName System.Drawing

$bmp = New-Object System.Drawing.Bitmap 256, 256
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.Clear([System.Drawing.Color]::FromArgb(25, 118, 210))  # Facebook blue

$font = New-Object System.Drawing.Font('Segoe UI', 100, [System.Drawing.FontStyle]::Bold)
$brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$sf = New-Object System.Drawing.StringFormat
$sf.Alignment = [System.Drawing.StringAlignment]::Center
$sf.LineAlignment = [System.Drawing.StringAlignment]::Center

$rect = New-Object System.Drawing.RectangleF(0, 0, 256, 256)
$gfx.DrawString('M', $font, $brush, $rect, $sf)

$bmp.Save('icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Save('tray-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)

# Create smaller version for tray (32x32)
$trayBmp = New-Object System.Drawing.Bitmap 32, 32
$trayGfx = [System.Drawing.Graphics]::FromImage($trayBmp)
$trayGfx.Clear([System.Drawing.Color]::FromArgb(25, 118, 210))
$trayFont = New-Object System.Drawing.Font('Segoe UI', 16, [System.Drawing.FontStyle]::Bold)
$trayRect = New-Object System.Drawing.RectangleF(0, 0, 32, 32)
$trayGfx.DrawString('M', $trayFont, $brush, $trayRect, $sf)
$trayBmp.Save('tray-icon.png', [System.Drawing.Imaging.ImageFormat]::Png)

# Save as ICO
$bmp.Save('icon.ico', [System.Drawing.Imaging.ImageFormat]::Icon)

Write-Host "Icons created successfully"