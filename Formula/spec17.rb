class Spec17 < Formula
  desc "CLI, MCP server and web dashboard for spec-driven-development evidence"
  homepage "https://spec17.dev"
  license :cannot_represent

  # Begin stable release
  version "0.5.0"

  on_macos do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.5.0/spec17-0.5.0-darwin-aarch64.tar.gz"
      sha256 "d24a4cedc4575f6a31a273e0e1f921e07f3ae17fe1962b0dbae783ecf9901119"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.5.0/spec17-0.5.0-linux-aarch64.tar.gz"
      sha256 "29ef716bee280380fd12db1bbb658b019b3a79b28dd71d3ef72a3e7961f67f80"
    end
    on_intel do
      url "https://github.com/kpavlov/spec17/releases/download/v0.5.0/spec17-0.5.0-linux-x86_64.tar.gz"
      sha256 "7b918c0ddeadf7a508282bf901cde4fe2ea50a12a1a1536abb73bd179f1fbf46"
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
