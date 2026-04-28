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

# Check for Git
if (!(Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "📦 Installing Git..."
    winget install Git.Git --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "✓ Git already installed"
}

# Clone the repository
$InstallDir = Join-Path $HOME "yacito"
if (!(Test-Path $InstallDir)) {
    Write-Host "📂 Cloning Yacito into $InstallDir..."
    git clone https://github.com/guidonaselli/yacito "$InstallDir"
}

Set-Location $InstallDir

Write-Host "🚀 Building Yacito (Production)... This may take a few minutes." -ForegroundColor Cyan
npm install
npm run setup:httpyac
npm run build:app

# Create Desktop Shortcut
try {
    $ExePath = Join-Path $PWD "src-tauri\target\release\yacito.exe"
    if (!(Test-Path $ExePath)) {
        # Fallback for different tauri build patterns if necessary
        $ExePath = (Get-ChildItem -Path "src-tauri\target\release\*.exe" | Select-Object -First 1).FullName
    }

    $DesktopPath = [Environment]::GetFolderPath("Desktop")
    $ShortcutPath = Join-Path $DesktopPath "Yacito.lnk"
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $ExePath
    $Shortcut.WorkingDirectory = $PWD
    $Shortcut.Description = "Yacito - Baby-easy httpYac GUI"
    $Shortcut.IconLocation = Join-Path $PWD "static\favicon.png"
    $Shortcut.Save()
    Write-Host "✅ Desktop shortcut 'Yacito' created!" -ForegroundColor Green
} catch {
    Write-Host "⚠️ Could not create desktop shortcut, but build finished." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host "The productive version of Yacito is now on your Desktop."
Write-Host "You can also run it by typing 'yacito' in your terminal (after restart)."
