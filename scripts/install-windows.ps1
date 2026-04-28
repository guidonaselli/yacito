Write-Host "🍼 Yacito Windows Quick Installer" -ForegroundColor Cyan
Write-Host "----------------------------"

# Check for Winget
if (!(Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Winget not found. Please install it from the Microsoft Store." -ForegroundColor Red
    return
}

# Rust
if (!(Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "🦀 Installing Rust..."
    winget install Rustlang.Rustup --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "✓ Rust already installed"
}

# Node.js
if (!(Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "📦 Installing Node.js LTS..."
    winget install OpenJS.NodeJS.LTS --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "✓ Node.js already installed"
}

Write-Host "🚀 Setting up project..."
npm install
npm run setup:httpyac

# Create Desktop Shortcut
try {
    $DesktopPath = [Environment]::GetFolderPath("Desktop")
    $ShortcutPath = Join-Path $DesktopPath "Yacito Dev.lnk"
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    # Launch via powershell to keep context, but could also point to a .bat
    $Shortcut.TargetPath = "powershell.exe"
    $Shortcut.Arguments = "-NoExit -Command `"cd '$PWD'; npm run dev:app`""
    $Shortcut.WorkingDirectory = $PWD
    $Shortcut.Description = "Yacito - Baby-easy httpYac GUI"
    $Shortcut.IconLocation = Join-Path $PWD "static\favicon.png"
    $Shortcut.Save()
    Write-Host "✅ Desktop shortcut 'Yacito Dev' created!" -ForegroundColor Green
} catch {
    Write-Host "⚠️ Could not create desktop shortcut, but installation finished." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host "IMPORTANT: You might need to RESTART your terminal for PATH changes to take effect." -ForegroundColor Yellow
Write-Host "Then run 'yacito' (if you added the alias) or use the Desktop shortcut."
