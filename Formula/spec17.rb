class Spec17 < Formula
  desc "CLI, MCP server and web dashboard for spec-driven-development evidence"
  homepage "https://spec17.dev"
  license :cannot_represent

  # Begin stable release
  version "0.3.0"

  on_macos do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.3.0/spec17-0.3.0-darwin-aarch64.tar.gz"
      sha256 "f64d549032459253c47072d84f786cb6d57a73036177d4d0adbed3d83f6a2c04"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.3.0/spec17-0.3.0-linux-aarch64.tar.gz"
      sha256 "8bbaf340eb237f8f4c5d2f5e54cc8e63af0f181fa4ebe4d1cfa84353245f909b"
    end
    on_intel do
      url "https://github.com/kpavlov/spec17/releases/download/v0.3.0/spec17-0.3.0-linux-x86_64.tar.gz"
      sha256 "a4abc999c22f865a1d71c3853eeca5e815aca710931c93163a92efe3cb52ee7d"
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
