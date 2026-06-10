#!/usr/bin/env bash
# deskbrid-install.sh — one-shot install: detect desktop, install deps, download binary
# Usage: bash <(curl -fsSL https://deskbrid.patchhive.dev/install.sh)
set -euo pipefail

REPO="https://github.com/coe0718/deskbrid"

# ── discover latest version from GitHub API ──
VER=""
if [[ -z "${DESKBRID_VERSION:-}" ]]; then
  VER="$(curl -fsSL "https://api.github.com/repos/coe0718/deskbrid/releases/latest" \
    | grep -oP '"tag_name":\s*"v\K[^"]+')" || true
fi
VER="${DESKBRID_VERSION:-${VER:-1.0.0}}"

# ── colors ──
RST='\033[0m'; RED='\033[0;31m'; GRN='\033[0;32m'; YLW='\033[0;33m'; CYN='\033[0;36m'
info()  { echo -e "  ${CYN}→${RST} $*"; }
ok()    { echo -e "  ${GRN}✓${RST} $*"; }
warn()  { echo -e "  ${YLW}⚠${RST} $*"; }
fail()  { echo -e "  ${RED}✗${RST} $*"; exit 1; }

# ── root guard ──
if [[ $EUID -eq 0 ]]; then
  fail "Don't run this as root. It uses sudo when needed."
fi

# ── sudo helper ──
sudocmd() {
  if ! sudo -n true 2>/dev/null; then
    echo
    warn "Some packages need root to install."
    echo -e "  ${YLW}↓${RST} Enter your sudo password when prompted."
    echo
  fi
  sudo "$@"
}

# ── detect distro ──
detect_distro() {
  if [[ -f /etc/os-release ]]; then
    . /etc/os-release
    DISTRO_ID="$ID"
    DISTRO_LIKE="${ID_LIKE:-}"
  else
    DISTRO_ID="unknown"
    DISTRO_LIKE=""
  fi
  info "Distro: ${CYN}${DISTRO_ID}${RST}"
}
detect_distro

# ── package manager ──
PKG_INSTALL=""
PKG_UPDATE=""
case "$DISTRO_ID" in
  ubuntu|debian|pop|linuxmint|elementary|zorin)
    PKG_INSTALL="sudo apt install -y"
    PKG_UPDATE="sudo apt update -qq"
    ;;
  arch|endeavouros|manjaro|arcolinux)
    PKG_INSTALL="sudo pacman -S --noconfirm"
    PKG_UPDATE="sudo pacman -Sy"
    ;;
  fedora|nobara)
    PKG_INSTALL="sudo dnf install -y"
    PKG_UPDATE="sudo dnf check-update -q || true"
    ;;
  opensuse*)
    PKG_INSTALL="sudo zypper install -y"
    PKG_UPDATE="sudo zypper refresh"
    ;;
  alpine)
    PKG_INSTALL="sudo apk add"
    PKG_UPDATE="sudo apk update"
    ;;
  *)
    warn "Unrecognised distro: $DISTRO_ID (trying apt as fallback)"
    PKG_INSTALL="sudo apt install -y"
    PKG_UPDATE="sudo apt update -qq"
    ;;
esac

# ── detect desktop environment ──
detect_de() {
  # Check session vars first
  local session="${XDG_SESSION_DESKTOP:-${DESKTOP_SESSION:-}}"
  session="${session,,}"

  if [[ "$session" == *"hyprland"* ]]; then
    echo "hyprland"
  elif [[ "$session" == *"kde"* || "$session" == *"plasma"* ]]; then
    echo "kde"
  elif [[ "$session" == *"gnome"* || "$session" == *"budgie"* ]]; then
    echo "gnome"
  elif [[ "$session" == *"xfce"* || "$session" == *"cinnamon"* || "$session" == *"mate"* || "$session" == *"i3"* || "$session" == *"sway"* || "$session" == *"openbox"* ]]; then
    echo "x11"
  elif [[ -n "$WAYLAND_DISPLAY" ]]; then
    echo "wayland-generic"
  elif [[ -n "$DISPLAY" ]]; then
    echo "x11"
  else
    echo "unknown"
  fi
}

DE="$(detect_de)"
info "Desktop: ${CYN}${DE}${RST}"

# ── dep maps ──
# Format: "package1 package2" for each distro family
apt_pkgs_common="socat libnotify-bin"
pacman_pkgs_common="socat libnotify"

declare -A apt_map=(
  ["gnome"]="grim wl-clipboard python3-gi gstreamer1.0-tools gstreamer1.0-pipewire xdg-desktop-portal xdg-desktop-portal-gnome"
  ["hyprland"]="grim wl-clipboard ydotool hyprland"
  ["kde"]="spectacle imagemagick ydotool qt6-tools"
  ["x11"]="xdotool wmctrl xclip imagemagick xprintidle"
  ["wayland-generic"]="grim wl-clipboard"
)

declare -A pacman_map=(
  ["gnome"]="grim wl-clipboard python-gobject gstreamer gst-plugin-pipewire xdg-desktop-portal xdg-desktop-portal-gnome"
  ["hyprland"]="grim wl-clipboard ydotool hyprland"
  ["kde"]="spectacle imagemagick ydotool qt6-tools"
  ["x11"]="xdotool wmctrl xclip imagemagick xprintidle"
  ["wayland-generic"]="grim wl-clipboard"
)

# ── check what's missing ──
need_install=()
check_dep() {
  if ! command -v "$1" &>/dev/null; then
    need_install+=("$1")
  fi
}

check_python_gi() {
  if ! python3 -c 'from gi.repository import GLib, Gio' &>/dev/null; then
    need_install+=("python3-gi")
  fi
}

check_gstreamer_pipewire() {
  if ! command -v gst-inspect-1.0 &>/dev/null || ! gst-inspect-1.0 pipewiresrc &>/dev/null; then
    need_install+=("gstreamer-pipewire")
  fi
}

# check specific tools, not just package names
check_dep "socat"
check_dep "notify-send"

case "$DE" in
  gnome)
    check_dep "grim"
    check_dep "gst-launch-1.0"
    check_dep "python3"
    check_python_gi
    check_gstreamer_pipewire
    check_dep "wl-copy"
    ;;
  hyprland)
    check_dep "grim"
    check_dep "wl-copy"
    check_dep "ydotool"
    check_dep "hyprctl"
    ;;
  kde)
    check_dep "spectacle"
    check_dep "convert"
    check_dep "ydotool"
    check_dep "qdbus6"
    ;;
  x11)
    check_dep "xdotool"
    check_dep "wmctrl"
    check_dep "xclip"
    check_dep "import"
    check_dep "xprintidle"
    ;;
  wayland-generic)
    check_dep "grim"
    check_dep "wl-copy"
    ;;
esac

# ── install ──
if [[ ${#need_install[@]} -eq 0 ]]; then
  ok "All dependencies already installed!"
else
  echo
  info "Missing packages: ${YLW}${need_install[*]}${RST}"
  echo

  # Map tool names → package names per distro
  pkg_list=()
  if [[ "$DISTRO_ID" == "arch" || "$DISTRO_ID" == "endeavouros" || "$DISTRO_ID" == "manjaro" || "$DISTRO_LIKE" == *"arch"* ]]; then
    pkg_list+=( $pacman_pkgs_common )
    pkg_list+=( ${pacman_map[$DE]:-} )
  else
    pkg_list+=( $apt_pkgs_common )
    pkg_list+=( ${apt_map[$DE]:-} )
  fi

  # Remove duplicates
  readarray -t pkg_list < <(printf '%s\n' "${pkg_list[@]}" | sort -u | tr '\n' ' ')

  info "Running: ${CYN}$PKG_UPDATE${RST}"
  $PKG_UPDATE

  info "Installing: ${CYN}${pkg_list[*]}${RST}"
  $PKG_INSTALL ${pkg_list[@]}
  ok "Dependencies installed!"
fi

# ── uinput permissions (Wayland backends) ──
if [[ "$DE" == "hyprland" || "$DE" == "kde" ]]; then
  if [[ ! -c /dev/uinput ]]; then
    warn "/dev/uinput not found — input device may be missing"
  elif [[ ! -r /dev/uinput ]]; then
    echo
    info "Setting up /dev/uinput permissions for ydotool..."
    sudocmd bash -c 'echo "KERNEL==\"uinput\", GROUP=\"input\", MODE=\"0660\"" > /etc/udev/rules.d/99-uinput.rules'
    sudocmd udevadm control --reload-rules
    sudocmd udevadm trigger
    sudocmd usermod -aG input "$USER"
    warn "You may need to log out/back in for the 'input' group to take effect."
    info "For now, you can: ${CYN}sudo chmod 666 /dev/uinput${RST}"
    ok "udev rule created!"
  else
    ok "/dev/uinput is accessible"
  fi
fi

# ── Backlight permissions (sysfs) ──
if [[ -d /sys/class/backlight ]] && [[ -n "$(ls -A /sys/class/backlight 2>/dev/null)" ]]; then
  BACKLIGHT_BRIGHTNESS=$(ls /sys/class/backlight/*/brightness 2>/dev/null | head -1)
  if [[ -n "$BACKLIGHT_BRIGHTNESS" ]] && [[ ! -w "$BACKLIGHT_BRIGHTNESS" ]]; then
    echo
    info "Setting up backlight permissions (video group)..."
    sudocmd bash -c 'echo "SUBSYSTEM==\"backlight\", RUN+=\"/bin/chgrp video /sys/class/backlight/%k/brightness\", RUN+=\"/bin/chmod g+w /sys/class/backlight/%k/brightness\"" > /etc/udev/rules.d/99-backlight.rules'
    sudocmd udevadm control --reload-rules
    sudocmd udevadm trigger
    sudocmd usermod -aG video "$USER"
    warn "You may need to log out/back in for the 'video' group to take effect."
    ok "Backlight udev rule created!"
  else
    ok "Backlight brightness is accessible"
  fi
fi

# ── download deskbrid binary ──
BIN_PATH="/usr/local/bin/deskbrid"
if command -v deskbrid &>/dev/null; then
  ok "deskbrid already installed at $(which deskbrid)"
else
  echo
  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64)  ARCH="x86_64-unknown-linux-gnu" ;;
    aarch64|arm64) ARCH="aarch64-unknown-linux-gnu" ;;
    *) fail "Unsupported architecture: $ARCH" ;;
  esac

  URL="${REPO}/releases/download/v${VER}/deskbrid-${ARCH}.tar.gz"
  SHA256_URL="${URL}.sha256"
  TMPDIR="$(mktemp -d)"

  info "Downloading deskbrid v${VER}..."
  curl -fsSL "$URL" -o "$TMPDIR/deskbrid.tar.gz" || fail "download failed: $URL"
  curl -fsSL "$SHA256_URL" -o "$TMPDIR/deskbrid.tar.gz.sha256" || fail "checksum download failed: $SHA256_URL"

  # Verify checksum before extracting
  EXPECTED=$(awk '{print $1}' "$TMPDIR/deskbrid.tar.gz.sha256")
  ACTUAL=$(sha256sum "$TMPDIR/deskbrid.tar.gz" | awk '{print $1}')
  [ "$EXPECTED" = "$ACTUAL" ] || fail "checksum mismatch

Expected: $EXPECTED
Got:      $ACTUAL"
  ok "Checksum verified — ${EXPECTED:0:16}..."

  tar -xzf "$TMPDIR/deskbrid.tar.gz" -C "$TMPDIR"

  sudocmd mv "$TMPDIR/deskbrid" "$BIN_PATH"
  sudocmd chmod +x "$BIN_PATH"
  rm -rf "$TMPDIR"
  ok "Installed deskbrid to ${CYN}${BIN_PATH}${RST}"
fi

# ── ydotoold autostart (Wayland backends) ──
if [[ "$DE" == "hyprland" || "$DE" == "kde" ]]; then
  if ! pgrep -x ydotoold &>/dev/null; then
    echo
    info "ydotoold isn't running. Setting up autostart..."
    mkdir -p "$HOME/.config/autostart"
    cat > "$HOME/.config/autostart/ydotoold.desktop" << 'EOF'
[Desktop Entry]
Type=Application
Name=ydotoold
Exec=ydotoold
Terminal=false
NoDisplay=true
EOF
    ok "ydotoold autostart created at ~/.config/autostart/ydotoold.desktop"
    warn "ydotoold not running — start it with: ${CYN}ydotoold &${RST}"
    warn "Or log out and back in for autostart to pick it up."
  else
    ok "ydotoold is already running"
  fi
fi

# ── done ──
echo
echo -e "  ${GRN}══════════════════════════════════════════${RST}"
echo -e "  ${GRN}  Deskbrid v${VER} ready!${RST}"
echo -e "  ${GRN}══════════════════════════════════════════${RST}"
echo
echo -e "  Start the daemon:  ${CYN}deskbrid daemon${RST}"
echo -e "  Check health:      ${CYN}deskbrid health${RST}"
echo -e "  Documentation:     ${CYN}${REPO}${RST}"
echo
echo -e "  ${CYN}Need an agent to control your desktop?${RST}"
echo -e "  Tell Tuck: ${YLW}\"Hey, install Deskbrid for me\"${RST}"
echo
