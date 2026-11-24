class Spec17Dev < Formula
  desc "Prebuilt development version of spec17 for Apple Silicon"
  homepage "https://spec17.dev"
  license :cannot_represent
  url "https://github.com/kpavlov/spec17/releases/download/edge/spec17-dev.20261124091448-darwin-aarch64.tar.gz"
  version "20261124091448"
  sha256 "62d79c3734273ddfd3106f4570afe2a1665f6bd0a0ae0aa132de3301bf0b647f"

  depends_on :macos
  depends_on arch: :arm64
  conflicts_with "spec17", because: "both install a spec17 binary"

  def install
    bin.install "spec17"
  end

  test do
    system "#{bin}/spec17", "--version"
  end
end
