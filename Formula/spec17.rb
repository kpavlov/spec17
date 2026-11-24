class Spec17 < Formula
  desc "CLI, MCP server and web dashboard for spec-driven-development evidence"
  homepage "https://spec17.dev"
  license :cannot_represent

  # Begin stable release
  version "0.2.0"

  on_macos do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.2.0/spec17-0.2.0-darwin-aarch64.tar.gz"
      sha256 "33bc9091157a9b3195e565d622241c037bbf391044358bd4e3b7685d2c04297b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/kpavlov/spec17/releases/download/v0.2.0/spec17-0.2.0-linux-aarch64.tar.gz"
      sha256 "630dd16c7176521652249d0448b82cde1fefc1d872e7e137dcc22e9a68e17ae4"
    end
    on_intel do
      url "https://github.com/kpavlov/spec17/releases/download/v0.2.0/spec17-0.2.0-linux-x86_64.tar.gz"
      sha256 "e769aeff411d90d97236163899fbd68638ff408c432dbcdd8ff17a997f81fbec"
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
