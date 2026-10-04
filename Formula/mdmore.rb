class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.2/mdmore-v0.1.2-aarch64-apple-darwin.tar.gz"
      sha256 "bc0dac7ada24e0a2616edb7b0f1c7977632d60b66f7cfb7428bbff232f09afdb"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.2/mdmore-v0.1.2-x86_64-apple-darwin.tar.gz"
      sha256 "20b4694b422190e0e0f53df7c04d29eefad184c08a116955d7ced4b4001cbd43"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.2/mdmore-v0.1.2-aarch64-unknown-linux-musl.tar.gz"
      sha256 "efc475772723abb6045d6947b9b82e104f0c7ff8edb64415181517dc8b1972f9"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.2/mdmore-v0.1.2-x86_64-unknown-linux-musl.tar.gz"
      sha256 "e8537719c911c00995c3a83a0aea2c11b23ab0dcc49c548c2067f72bd3d53289"
    end
  end

  def install
    bin.install "mdmore"
    doc.install "README.md", "docs"
  end

  test do
    assert_match "mdmore #{version}", shell_output("#{bin}/mdmore --version")
    (testpath/"sample.md").write("# Sample\n\n**hello** `world`\n")
    assert_equal "# Sample\n\nhello world\n\n", shell_output("#{bin}/mdmore --plain sample.md")
  end
end
