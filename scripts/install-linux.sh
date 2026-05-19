#!/usr/bin/env bash
set -euo pipefail

APP_NAME="Yacito"
REPO_URL="https://github.com/guidonaselli/yacito"
DEFAULT_INSTALL_DIR="${HOME}/work/yacito"
INSTALL_DIR="${DEFAULT_INSTALL_DIR}"
API_HTTP_DIR="${HTTPYAC_API_HTTP_DIR:-}"
SKIP_BUILD=0
BINARY_PATH=""

usage() {
  cat <<'EOF'
Usage: bash scripts/install-linux.sh [options]

Options:
  --dir <path>            Installation directory (default: ~/work/yacito)
  --api-http-dir <path>   Default api-http directory passed to Yacito launcher
  --skip-build            Clone/update and install deps, but skip desktop bundle build
  -h, --help              Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --api-http-dir)
      API_HTTP_DIR="$2"
      shift 2
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

say() {
  printf '\n%s\n' "$1"
}

ensure_local_bin_on_path() {
  local shell_rc="${HOME}/.bashrc"

  mkdir -p "${HOME}/.local/bin"

  if [[ ":${PATH}:" != *":${HOME}/.local/bin:"* ]]; then
    if [[ -f "${shell_rc}" ]] && grep -Fq 'export PATH="$HOME/.local/bin:$PATH"' "${shell_rc}"; then
      return
    fi
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "${shell_rc}"
    echo "✓ Added ~/.local/bin to PATH in ${shell_rc}"
  fi
}

run_pkg_install() {
  local pkgs=("$@")

  if need_cmd pacman; then
    sudo pacman -Sy --needed --noconfirm "${pkgs[@]}"
  elif need_cmd apt-get; then
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends "${pkgs[@]}"
  elif need_cmd dnf; then
    sudo dnf install -y "${pkgs[@]}"
  elif need_cmd zypper; then
    sudo zypper install -y "${pkgs[@]}"
  else
    echo "Unsupported package manager. Install dependencies manually." >&2
    return 1
  fi
}

install_system_deps() {
  say "📦 Installing Linux dependencies..."

  if need_cmd pacman; then
    run_pkg_install \
      webkit2gtk-4.1 \
      base-devel \
      curl \
      wget \
      file \
      openssl \
      libayatana-appindicator \
      librsvg \
      patchelf \
      rpm-tools \
      xdg-utils \
      git \
      nodejs \
      npm
  elif need_cmd apt-get; then
    run_pkg_install \
      libwebkit2gtk-4.1-dev \
      build-essential \
      curl \
      wget \
      file \
      libxdo-dev \
      libssl-dev \
      libayatana-appindicator3-dev \
      librsvg2-dev \
      patchelf \
      rpm \
      xdg-utils \
      git \
      nodejs \
      npm
  elif need_cmd dnf; then
    run_pkg_install \
      webkit2gtk4.1-devel \
      gcc-c++ \
      make \
      curl \
      wget \
      file \
      openssl-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel \
      patchelf \
      rpm-build \
      xdg-utils \
      git \
      nodejs \
      npm
  elif need_cmd zypper; then
    run_pkg_install \
      webkit2gtk3-soup3-devel \
      gcc-c++ \
      make \
      curl \
      wget \
      file \
      libopenssl-devel \
      libayatana-appindicator3-devel \
      librsvg-devel \
      patchelf \
      rpm-build \
      xdg-utils \
      git \
      nodejs \
      npm
  fi
}

ensure_rust() {
  if need_cmd rustc && need_cmd cargo; then
    echo "✓ Rust already installed"
    return
  fi

  say "🦀 Installing Rust toolchain..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1090
  source "${HOME}/.cargo/env"
}

validate_inputs() {
  if [[ -n "${API_HTTP_DIR}" && ! -d "${API_HTTP_DIR}" ]]; then
    echo "api-http directory not found: ${API_HTTP_DIR}" >&2
    exit 1
  fi
}

clone_or_update_repo() {
  mkdir -p "$(dirname "${INSTALL_DIR}")"

  if [[ ! -d "${INSTALL_DIR}/.git" ]]; then
    say "📂 Cloning ${APP_NAME} into ${INSTALL_DIR}..."
    git clone "${REPO_URL}" "${INSTALL_DIR}"
    return
  fi

  say "🔄 Updating existing repository in ${INSTALL_DIR}..."
  git -C "${INSTALL_DIR}" fetch origin
  git -C "${INSTALL_DIR}" pull --ff-only origin main
}

write_launcher() {
  local launcher_dir="${HOME}/.local/bin"
  local launcher_path="${launcher_dir}/yacito"
  local binary_path="${INSTALL_DIR}/src-tauri/target/release/yacito"

  if [[ ! -x "${binary_path}" ]]; then
    echo "⚠️  Skipping launcher creation because the binary was not built yet: ${binary_path}"
    return
  fi

  mkdir -p "${launcher_dir}"

  cat > "${launcher_path}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
export HTTPYAC_API_HTTP_DIR="${API_HTTP_DIR}"
exec "${binary_path}" "\$@"
EOF

  chmod +x "${launcher_path}"

  mkdir -p "${HOME}/.local/share/applications"
  cat > "${HOME}/.local/share/applications/yacito.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Yacito
Exec=${launcher_path}
Icon=${INSTALL_DIR}/src-tauri/icons/128x128.png
Terminal=false
Categories=Development;
EOF
}

build_project() {
  cd "${INSTALL_DIR}"

  say "🚀 Installing Node dependencies..."
  npm install

  say "🩺 Running doctor..."
  npm run doctor

  if [[ "${SKIP_BUILD}" -eq 1 ]]; then
    echo "↷ Skipping bundle build (--skip-build)"
    return
  fi

  say "🏗️ Building Linux desktop bundles..."
  npm run build:app:linux
  BINARY_PATH="${INSTALL_DIR}/src-tauri/target/release/yacito"
}

say "🍼 ${APP_NAME} Linux Installer"
echo "Repo: ${REPO_URL}"
echo "Dir : ${INSTALL_DIR}"
if [[ -n "${API_HTTP_DIR}" ]]; then
  echo "API : ${API_HTTP_DIR}"
fi

validate_inputs
ensure_local_bin_on_path
install_system_deps
ensure_rust
clone_or_update_repo
build_project
write_launcher

say "✨ Installation complete!"
if [[ -x "${INSTALL_DIR}/src-tauri/target/release/yacito" ]]; then
  echo "Command available at: ~/.local/bin/yacito"
else
  echo "Binary not built yet. Run: cd ${INSTALL_DIR} && npm run build:app:linux"
fi
if [[ -n "${API_HTTP_DIR}" ]]; then
  echo "Default api-http dir configured: ${API_HTTP_DIR}"
else
  echo "Tip: pass --api-http-dir /path/to/api-http to preconfigure your workspace."
fi
