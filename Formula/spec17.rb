class Spec17 < Formula
  desc "CLI, MCP server and web dashboard for spec-driven-development evidence"
  homepage "https://spec17.dev"
  license :cannot_represent

  # Begin stable release
  version "0.6.0"

  on_macos do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.6.0/spec17-0.6.0-darwin-aarch64.tar.gz"
      sha256 "51d6711a7160c5fa1c4d8f8275f1dea2b48584f2240d3f6f5b8a04bf1933f16d"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.6.0/spec17-0.6.0-linux-aarch64.tar.gz"
      sha256 "deea2af49f2996b91b91c0a53e17604f01f2b1a56b0df6b148224b1e6db58bb6"
    end
    on_intel do
      url "https://github.com/kpavlov/spec17/releases/download/v0.6.0/spec17-0.6.0-linux-x86_64.tar.gz"
      sha256 "55737c736d4c281763cbc3cecfc5eb348fc87373930b7ae8cb8c87e1c44a129e"
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
