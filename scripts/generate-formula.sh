#!/usr/bin/env sh
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <version> <path-to-SHA256SUMS>" >&2
  exit 1
fi

version="$1"
sums_file="$2"
repo="${SWITCHLOOM_REPO:-instructa/switchloom}"
base_url="https://github.com/$repo/releases/download/v$version"

sha_for() {
  asset="switchloom-$1.tar.gz"
  sha="$(awk -v asset="$asset" '$2 == asset {print $1}' "$sums_file" | head -n 1)"
  if [ -z "$sha" ]; then
    echo "checksum for $asset not found in $sums_file" >&2
    exit 1
  fi
  echo "$sha"
}

darwin_arm64="$(sha_for darwin-arm64)"
darwin_x86_64="$(sha_for darwin-x86_64)"
linux_arm64="$(sha_for linux-arm64)"
linux_x86_64="$(sha_for linux-x86_64)"

cat <<EOF
class Switchloom < Formula
  desc "Deterministic model routing for coding agents"
  homepage "https://github.com/$repo"
  version "$version"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "$base_url/switchloom-darwin-arm64.tar.gz"
      sha256 "$darwin_arm64"
    else
      url "$base_url/switchloom-darwin-x86_64.tar.gz"
      sha256 "$darwin_x86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "$base_url/switchloom-linux-arm64.tar.gz"
      sha256 "$linux_arm64"
    else
      url "$base_url/switchloom-linux-x86_64.tar.gz"
      sha256 "$linux_x86_64"
    end
  end

  def install
    bin.install "model-routing"
    bin.install_symlink "model-routing" => "switchloom"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/switchloom --version")
  end
end
EOF
