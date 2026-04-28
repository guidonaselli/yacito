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
} else {
    Write-Host "🔄 Updating existing Yacito repository..."
    Set-Location $InstallDir
    git fetch origin
    git reset --hard origin/main
}

Set-Location $InstallDir

Write-Host "🚀 Building Yacito (Production)... This may take a few minutes." -ForegroundColor Cyan
npm install
npm run setup:httpyac
npm run build:app

# Create Desktop Shortcut
try {
    # Tauri uses productName for the executable name
    $ExePath = Join-Path $PWD "src-tauri\target\release\Yacito.exe"
    if (!(Test-Path $ExePath)) {
        # Fallback: search for any exe in release folder if productName didn't match
        $ExePath = (Get-ChildItem -Path "src-tauri\target\release\*.exe" | Select-Object -First 1).FullName
    }

    if ($ExePath) {
        $DesktopPath = [Environment]::GetFolderPath("Desktop")
        $ShortcutPath = Join-Path $DesktopPath "Yacito.lnk"
        $WshShell = New-Object -ComObject WScript.Shell
        $Shortcut = $WshShell.CreateShortcut($ShortcutPath)
        $Shortcut.TargetPath = $ExePath
        $Shortcut.WorkingDirectory = $PWD
        $Shortcut.Description = "Yacito - Baby-easy httpYac GUI"
        # Use the EXE itself for the icon, index 0 is the primary app icon
        $Shortcut.IconLocation = "$ExePath,0"
        $Shortcut.Save()
        Write-Host "✅ Desktop shortcut 'Yacito' created!" -ForegroundColor Green
    } else {
        Write-Host "⚠️ Could not find the Yacito executable to create a shortcut." -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️ Could not create desktop shortcut, but build finished." -ForegroundColor Yellow
}

# Add persistent alias to PowerShell Profile
try {
    $ProfileDir = Split-Path $PROFILE -Parent
    if (!(Test-Path $ProfileDir)) { New-Item -Path $ProfileDir -ItemType Directory -Force }
    if (!(Test-Path $PROFILE)) { New-Item -Path $PROFILE -ItemType File -Force }
    
    $AliasLine = "function yacito { & '$ExePath' `$args }"
    if (!(Select-String -Path $PROFILE -Pattern "function yacito")) {
        Add-Content -Path $PROFILE -Value "`n$AliasLine"
        Write-Host "✅ Alias 'yacito' added to your PowerShell Profile!" -ForegroundColor Green
    }
} catch {
    Write-Host "⚠️ Could not add alias to Profile, but you can still use the Desktop shortcut." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host "The productive version of Yacito is now on your Desktop."
Write-Host "Please RESTART your terminal to use the 'yacito' command."
