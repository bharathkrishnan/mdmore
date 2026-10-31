class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.4/mdmore-v0.1.4-aarch64-apple-darwin.tar.gz"
      sha256 "a0b73ac4b73c89f9eff02bdbc07b95414fe25c562e5b462e323cb71d915aaf73"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.4/mdmore-v0.1.4-x86_64-apple-darwin.tar.gz"
      sha256 "8bf43981ade9c50597e8af15315f5b78a3b14b1badf45985c69af9def30a787c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.4/mdmore-v0.1.4-aarch64-unknown-linux-musl.tar.gz"
      sha256 "034af11fe9602f9c2ce25cb2bfb704cd38081ebb4029b13a92b31864a70436ad"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.4/mdmore-v0.1.4-x86_64-unknown-linux-musl.tar.gz"
      sha256 "3fe519e3e06e2f39cf782291a03c3d762f192c9397945eba714be4ce466fe753"
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
