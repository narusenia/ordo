#!/bin/sh
set -e

REPO="narusenia/ordo"
INSTALL_DIR="${ORDO_INSTALL_DIR:-/usr/local/bin}"

get_arch() {
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64) echo "x86_64" ;;
    aarch64|arm64) echo "aarch64" ;;
    *) echo "unsupported architecture: $arch" >&2; exit 1 ;;
  esac
}

get_os() {
  os=$(uname -s)
  case "$os" in
    Linux) echo "linux" ;;
    Darwin) echo "macos" ;;
    *) echo "unsupported OS: $os" >&2; exit 1 ;;
  esac
}

main() {
  os=$(get_os)
  arch=$(get_arch)
  artifact="ordo-${os}-${arch}"

  latest=$(curl -sI "https://github.com/${REPO}/releases/latest" | grep -i ^location: | sed 's/.*tag\///' | tr -d '\r')

  if [ -z "$latest" ]; then
    echo "error: could not determine latest version" >&2
    exit 1
  fi

  url="https://github.com/${REPO}/releases/download/${latest}/${artifact}"

  echo "Installing ordo ${latest} (${os}/${arch})..."
  echo "  ${url}"

  tmpdir=$(mktemp -d)
  trap 'rm -rf "$tmpdir"' EXIT

  curl -fsSL "$url" -o "${tmpdir}/ordo"
  chmod +x "${tmpdir}/ordo"

  if [ -w "$INSTALL_DIR" ]; then
    mv "${tmpdir}/ordo" "${INSTALL_DIR}/ordo"
  else
    echo "  sudo required to install to ${INSTALL_DIR}"
    sudo mv "${tmpdir}/ordo" "${INSTALL_DIR}/ordo"
  fi

  echo "  installed to ${INSTALL_DIR}/ordo"
  echo ""
  ordo --version 2>/dev/null || echo "  run: ordo --help"
}

main
