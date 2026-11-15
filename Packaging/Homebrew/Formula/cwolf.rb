class Cwolf < Formula
  desc "Dual-pane terminal file manager with Vim-style controls and media previews"
  homepage "https://github.com/max-wolf-cpp/commander-wolf"
  url "https://github.com/max-wolf-cpp/commander-wolf/releases/download/v0.1.0/commander-wolf-0.1.0-macos-arm64.tar.gz"
  sha256 "4581d919a137c8edc091ac63c6c16465eb111930c87706e8973e89d21ca39ba8"
  license "MIT"

  depends_on arch: :arm64
  depends_on macos: :sonoma

  # Preserve the signed release and its executable-relative library layout.
  preserve_rpath
  skip_clean "libexec"

  def install
    libexec.install Dir["*"]
    bin.write_exec_script libexec/"cwolf"
  end

  def caveats
    <<~EOS
      Launch cwolf from a terminal.
      A Nerd Font is recommended for the default icons.
    EOS
  end

  test do
    assert_match "cwolf: usage: cwolf [--clean] [--protocol PROTOCOL] [path]",
                 shell_output("#{bin}/cwolf __package_probe_one__ __package_probe_two__ 2>&1", 1)
  end
end
