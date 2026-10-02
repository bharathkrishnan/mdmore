class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "7ad00bb5309c4ac8515193ca3b022528e00d44922f66f8d67fc898eebc39af17"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "3e46b2ea87b3a65657b3d8a5017595f9839bda76ca2d0f9a307bddea530df028"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "827cab941884882fddd53a6ea68873aa0bca72e93d660e3b5ad46a29799de10a"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "c355dbddcf47a4820453f1e3ceed6dd8027b2c2f4f82d5c9c61cdf5adc994a6c"
    end
  end

  def install
    bin.install "mdmore"
    doc.install "README.md"
  end

  test do
    assert_match "mdmore #{version}", shell_output("#{bin}/mdmore --version")
    (testpath/"sample.md").write("# Sample\n\n**hello** `world`\n")
    assert_equal "# Sample\n\nhello world\n\n", shell_output("#{bin}/mdmore --plain sample.md")
  end
end
