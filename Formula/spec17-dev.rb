class Spec17Dev < Formula
  desc "Prebuilt development version of spec17 for Apple Silicon"
  homepage "https://spec17.dev"
  license :cannot_represent
  url "https://github.com/kpavlov/spec17/releases/download/edge/spec17-dev.20261126214331-darwin-aarch64.tar.gz"
  version "20261126214331"
  sha256 "21616bc81760fc8c75d06be8695d46e0141ac9d4697737c79058feb236e77095"

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
