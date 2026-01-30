class Picoflow < Formula
  desc "Lightweight DAG workflow orchestrator for edge devices"
  homepage "https://github.com/zoza1982/picoflow"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/zoza1982/picoflow/releases/download/v0.1.0/picoflow-v0.1.0-darwin-arm64.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_ARM64"
    else
      url "https://github.com/zoza1982/picoflow/releases/download/v0.1.0/picoflow-v0.1.0-darwin-x86_64.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      if Hardware::CPU.is_64_bit?
        url "https://github.com/zoza1982/picoflow/releases/download/v0.1.0/picoflow-v0.1.0-arm64-linux.tar.gz"
        sha256 "PLACEHOLDER_ARM64_LINUX"
      else
        url "https://github.com/zoza1982/picoflow/releases/download/v0.1.0/picoflow-v0.1.0-arm32-linux.tar.gz"
        sha256 "PLACEHOLDER_ARM32_LINUX"
      end
    else
      url "https://github.com/zoza1982/picoflow/releases/download/v0.1.0/picoflow-v0.1.0-x86_64-linux.tar.gz"
      sha256 "PLACEHOLDER_X86_64_LINUX"
    end
  end

  def install
    bin.install "picoflow"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/picoflow --version")
  end
end
