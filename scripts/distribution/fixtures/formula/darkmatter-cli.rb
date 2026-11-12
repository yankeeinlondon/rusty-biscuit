class DarkmatterCli < Formula
  desc "A themed markdown renderer for both console (escape-codes) and the browser (HTML)."
  homepage "https://github.com/yankeeinlondon/rusty-biscuit/tree/main/darkmatter"
  version "0.1.0"
  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/yankeeinlondon/rusty-biscuit/releases/download/darkmatter-cli-v0.1.0/darkmatter-cli-aarch64-apple-darwin.tar.xz"
      sha256 "e09ccfa1a932293ba60d43fcceb2f3e5911fd5aa84869afd06dbc02e625a5ee8"
    end
    if Hardware::CPU.intel?
      url "https://github.com/yankeeinlondon/rusty-biscuit/releases/download/darkmatter-cli-v0.1.0/darkmatter-cli-x86_64-apple-darwin.tar.xz"
    end
  end
  if OS.linux?
    if Hardware::CPU.arm?
      url "https://github.com/yankeeinlondon/rusty-biscuit/releases/download/darkmatter-cli-v0.1.0/darkmatter-cli-aarch64-unknown-linux-gnu.tar.xz"
    end
    if Hardware::CPU.intel?
      url "https://github.com/yankeeinlondon/rusty-biscuit/releases/download/darkmatter-cli-v0.1.0/darkmatter-cli-x86_64-unknown-linux-gnu.tar.xz"
    end
  end
  license "AGPL-3.0-only"

  BINARY_ALIASES = {
    "aarch64-apple-darwin": {},
    "aarch64-unknown-linux-gnu": {},
    "x86_64-apple-darwin": {},
    "x86_64-unknown-linux-gnu": {}
  }

  def target_triple
    cpu = Hardware::CPU.arm? ? "aarch64" : "x86_64"
    os = OS.mac? ? "apple-darwin" : "unknown-linux-gnu"

    "#{cpu}-#{os}"
  end

  def install_binary_aliases!
    BINARY_ALIASES[target_triple.to_sym].each do |source, dests|
      dests.each do |dest|
        bin.install_symlink bin/source.to_s => dest
      end
    end
  end

  def install
    if OS.mac? && Hardware::CPU.arm?
      bin.install "md"
    end
    if OS.mac? && Hardware::CPU.intel?
      bin.install "md"
    end
    if OS.linux? && Hardware::CPU.arm?
      bin.install "md"
    end
    if OS.linux? && Hardware::CPU.intel?
      bin.install "md"
    end

    install_binary_aliases!

    # Homebrew will automatically install these, so we don't need to do that
    doc_files = Dir["README.*", "readme.*", "LICENSE", "LICENSE.*", "CHANGELOG.*"]
    leftover_contents = Dir["*"] - doc_files

    # Install any leftover files in pkgshare; these are probably config or
    # sample files.
    pkgshare.install(*leftover_contents) unless leftover_contents.empty?
  end
end
