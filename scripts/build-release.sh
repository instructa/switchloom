#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/.."

version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"

detect_target() {
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"

  case "$os" in
    darwin) os="darwin" ;;
    linux) os="linux" ;;
    *)
      echo "unsupported OS: $os" >&2
      exit 1
      ;;
  esac

  case "$arch" in
    arm64 | aarch64) arch="arm64" ;;
    x86_64 | amd64) arch="x86_64" ;;
    *)
      echo "unsupported architecture: $arch" >&2
      exit 1
      ;;
  esac

  echo "$os-$arch"
}

sha256_tool() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$@"
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$@"
  else
    echo "shasum or sha256sum is required" >&2
    exit 1
  fi
}

target="${SWITCHLOOM_TARGET:-$(detect_target)}"
cargo_target="${SWITCHLOOM_CARGO_TARGET:-}"
target_dir="dist/switchloom-$version"
asset="switchloom-$target.tar.gz"

rm -rf "$target_dir" "dist/$asset"
mkdir -p "$target_dir"

if [ -n "$cargo_target" ]; then
  cargo build --release --locked --target "$cargo_target"
  built_bin="target/$cargo_target/release/model-routing"
else
  cargo build --release --locked
  built_bin="target/release/model-routing"
fi

cp "$built_bin" "$target_dir/model-routing"
cp README.md LICENSE "$target_dir/"

(
  cd "$target_dir"
  sha256_tool model-routing README.md LICENSE > SHA256SUMS
)

(
  cd "$target_dir"
  tar -czf "../$asset" model-routing README.md LICENSE SHA256SUMS
)

echo "release artifact: dist/$asset"
