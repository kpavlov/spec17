class Spec17 < Formula
  desc "CLI, MCP server and web dashboard for spec-driven-development evidence"
  homepage "https://spec17.dev"
  license :cannot_represent

  # Begin stable release
  version "0.4.0"

  on_macos do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.4.0/spec17-0.4.0-darwin-aarch64.tar.gz"
      sha256 "62d86ff65cd24674046f71a90955ba44dd445b9fa2c8b6020f9f3a0e80095105"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.4.0/spec17-0.4.0-linux-aarch64.tar.gz"
      sha256 "e70e109dcc8bb2804b9d8a0037fd15e0b2cdc7e7bea3287acb19591cea92b06f"
    end
    on_intel do
      url "https://github.com/kpavlov/spec17/releases/download/v0.4.0/spec17-0.4.0-linux-x86_64.tar.gz"
      sha256 "c22c797eca615b0c907f5aa4ac3c14580cf26fc5523cb68a50847c21aa2c7e96"
    end
  end

  # End stable release

  conflicts_with "spec17-dev", because: "both install a spec17 binary"

  # (NFR-13) HEAD is available before the first tagged release.
  head do
    url "https://github.com/kpavlov/spec17.git", branch: "main"
    depends_on "node" => :build
    depends_on "pnpm" => :build
    depends_on "rust" => :build
  end

  def install
    if build.head?
      cd "web" do
        system "pnpm", "install", "--frozen-lockfile", "--store-dir", buildpath/".pnpm-store"
        system "pnpm", "run", "build"
      end
      ENV["SKIP_WEB_BUILD"] = "1"
      system "cargo", "install", *std_cargo_args(path: "server")
    else
      bin.install "spec17"
    end
  end

  test do
    system "#{bin}/spec17", "--version"
  end
end
