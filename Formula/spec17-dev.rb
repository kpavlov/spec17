class Spec17Dev < Formula
  desc "Prebuilt development version of spec17 for Apple Silicon"
  homepage "https://spec17.dev"
  license :cannot_represent
  url "https://github.com/kpavlov/spec17/releases/download/edge/spec17-dev.20261121122323-darwin-aarch64.tar.gz"
  version "20261121122323"
  sha256 "2c4da5dacaf66fd84725e90747aabf859cf5e2640b375827543153e4bbc4e325"

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
