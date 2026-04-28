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

Write-Host ""
Write-Host "✨ Installation complete!" -ForegroundColor Green
Write-Host "IMPORTANT: You might need to RESTART your terminal for PATH changes to take effect." -ForegroundColor Yellow
Write-Host "Then run 'npm run dev:app' to start Yacito."
