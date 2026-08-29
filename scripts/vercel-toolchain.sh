#!/usr/bin/env bash
set -euo pipefail

# Vercel's Linux build image is x86_64. Keep this bootstrap self-contained so the
# install/build commands in vercel.json remain short and below Vercel's schema limit.
readonly NODE_VERSION="22.12.0"
readonly NODE_SHA256="22982235e1b71fa8850f82edd09cdae7e3f32df1764a9ec298c72d25ef2c164f"
readonly RUST_VERSION="1.97.1"
readonly RUSTUP_VERSION="1.28.2"
readonly RUSTUP_SHA256="20a06e644b0d9bd2fbdbfd52d42540bdde820ea7df86e92e533c073da0cdd43c"
readonly WASM_PACK_VERSION="0.15.0"
readonly WASM_PACK_SHA256="c09f971ecaed9a2efc80fdcea7a00ef6b53c7fadc8c57d1f61b53a6aa66b668a"

readonly TOOL_ROOT="${HOME}/.cache/limen-vercel-toolchain/${NODE_VERSION}-${RUST_VERSION}-${WASM_PACK_VERSION}"
readonly NODE_ROOT="${TOOL_ROOT}/node-v${NODE_VERSION}-linux-x64"
readonly RUSTUP_HOME="${TOOL_ROOT}/rustup-home"
readonly CARGO_HOME="${TOOL_ROOT}/cargo-home"
readonly RUSTUP_BIN="${TOOL_ROOT}/rustup-bin/rustup"
readonly WASM_PACK_BIN="${TOOL_ROOT}/bin/wasm-pack"

die() {
  printf 'Limen Vercel toolchain setup failed: %s\n' "$1" >&2
  exit 1
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d ' ' -f 1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | cut -d ' ' -f 1
  else
    die "sha256sum or shasum is required for artifact verification"
  fi
}

download_verified() {
  local url="$1"
  local expected_sha256="$2"
  local destination="$3"
  local temporary="${destination}.part"
  local actual_sha256

  mkdir -p "$(dirname "$destination")"
  if [[ -f "$destination" ]]; then
    actual_sha256="$(sha256_file "$destination")"
    [[ "$actual_sha256" == "$expected_sha256" ]] ||
      die "checksum mismatch for cached artifact $(basename "$destination")"
    return
  fi

  rm -f "$temporary"
  curl --fail --location --silent --show-error --retry 3 --output "$temporary" "$url"
  actual_sha256="$(sha256_file "$temporary")"
  [[ "$actual_sha256" == "$expected_sha256" ]] ||
    die "checksum mismatch for downloaded artifact $(basename "$destination")"
  mv "$temporary" "$destination"
}

ensure_node() {
  local archive="${TOOL_ROOT}/node-v${NODE_VERSION}-linux-x64.tar.xz"

  download_verified \
    "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-x64.tar.xz" \
    "$NODE_SHA256" \
    "$archive"

  if [[ ! -x "${NODE_ROOT}/bin/node" ]] ||
    [[ "$("${NODE_ROOT}/bin/node" --version)" != "v${NODE_VERSION}" ]]; then
    rm -rf "${TOOL_ROOT}/node-extract"
    mkdir -p "${TOOL_ROOT}/node-extract"
    tar -xJf "$archive" -C "${TOOL_ROOT}/node-extract"
    rm -rf "$NODE_ROOT"
    mv "${TOOL_ROOT}/node-extract/node-v${NODE_VERSION}-linux-x64" "$NODE_ROOT"
    rmdir "${TOOL_ROOT}/node-extract"
  fi

  [[ "$("${NODE_ROOT}/bin/node" --version)" == "v${NODE_VERSION}" ]] ||
    die "Node version check failed"
}

ensure_rust() {
  local rustup_installer="${TOOL_ROOT}/rustup-init"

  download_verified \
    "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" \
    "$RUSTUP_SHA256" \
    "$rustup_installer"
  chmod 0755 "$rustup_installer"
  mkdir -p "$TOOL_ROOT/rustup-bin" "$RUSTUP_HOME" "$CARGO_HOME"

  if [[ ! -x "$RUSTUP_BIN" ]]; then
    env RUSTUP_HOME="$RUSTUP_HOME" CARGO_HOME="$CARGO_HOME" "$rustup_installer" \
      -y \
      --default-toolchain none \
      --profile minimal \
      --no-modify-path \
      --install-dir "${TOOL_ROOT}/rustup-bin"
  fi

  export RUSTUP_HOME CARGO_HOME
  export PATH="${TOOL_ROOT}/rustup-bin:${CARGO_HOME}/bin:${NODE_ROOT}/bin:${TOOL_ROOT}/bin:${PATH}"

  local installed_rustc
  installed_rustc="$("$RUSTUP_BIN" run "$RUST_VERSION" rustc --version 2>/dev/null || true)"
  if [[ "$installed_rustc" != "rustc ${RUST_VERSION}"* ]]; then
    "$RUSTUP_BIN" toolchain install "$RUST_VERSION" \
      --profile minimal \
      --component rustfmt \
      --component clippy \
      --target wasm32-unknown-unknown \
      --no-self-update
  else
    "$RUSTUP_BIN" component add rustfmt clippy --toolchain "$RUST_VERSION"
    "$RUSTUP_BIN" target add wasm32-unknown-unknown --toolchain "$RUST_VERSION"
  fi

  [[ "$("$RUSTUP_BIN" run "$RUST_VERSION" rustc --version)" == "rustc ${RUST_VERSION}"* ]] ||
    die "Rust version check failed"
  local installed_targets
  installed_targets="$("$RUSTUP_BIN" target list --toolchain "$RUST_VERSION" --installed)"
  [[ $'\n'"$installed_targets"$'\n' == *$'\nwasm32-unknown-unknown\n'* ]] ||
    die "wasm32-unknown-unknown target check failed"
}

ensure_wasm_pack() {
  local archive="${TOOL_ROOT}/wasm-pack-v${WASM_PACK_VERSION}-x86_64-unknown-linux-musl.tar.gz"

  download_verified \
    "https://github.com/wasm-bindgen/wasm-pack/releases/download/v${WASM_PACK_VERSION}/wasm-pack-v${WASM_PACK_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    "$WASM_PACK_SHA256" \
    "$archive"
  mkdir -p "${TOOL_ROOT}/bin"

  if [[ ! -x "$WASM_PACK_BIN" ]]; then
    rm -rf "${TOOL_ROOT}/wasm-pack-extract"
    mkdir -p "${TOOL_ROOT}/wasm-pack-extract"
    tar -xzf "$archive" -C "${TOOL_ROOT}/wasm-pack-extract"
    mv "${TOOL_ROOT}/wasm-pack-extract/wasm-pack-v${WASM_PACK_VERSION}-x86_64-unknown-linux-musl/wasm-pack" \
      "$WASM_PACK_BIN"
    rm -rf "${TOOL_ROOT}/wasm-pack-extract"
  fi

  chmod 0755 "$WASM_PACK_BIN"
  [[ "$("$WASM_PACK_BIN" --version)" == "wasm-pack ${WASM_PACK_VERSION}" ]] ||
    die "wasm-pack version check failed"
}

setup_toolchain() {
  [[ "$(uname -s)" == "Linux" && "$(uname -m)" == "x86_64" ]] ||
    die "this pinned bootstrap targets Vercel's x86_64 Linux build image"
  ensure_node
  ensure_rust
  ensure_wasm_pack
}

case "${1:-}" in
  install)
    setup_toolchain
    export PATH="${NODE_ROOT}/bin:${TOOL_ROOT}/rustup-bin:${CARGO_HOME}/bin:${TOOL_ROOT}/bin:${PATH}"
    cd web && npm ci
    ;;
  build)
    setup_toolchain
    export PATH="${NODE_ROOT}/bin:${TOOL_ROOT}/rustup-bin:${CARGO_HOME}/bin:${TOOL_ROOT}/bin:${PATH}"
    cd web && npm run build
    ;;
  *)
    die "usage: $0 {install|build}"
    ;;
esac
