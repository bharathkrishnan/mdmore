class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "20748f24d86790b54f9b1e954f3bf161e5cdbe6e459040481d6f33957c9681dc"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "a37c0bd4e63b7ecd6a6bd198948bdd3c228c1846ff87645b51af19ff9b3b6b4b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-aarch64-unknown-linux-musl.tar.gz"
      sha256 "9056906b367a042fff0ac5ef09e15029f48d9d0dfb5f2503eaef5eca639959a5"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.0/mdmore-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
      sha256 "4c54e6903a7950db4574cd7e5e2e50722b4cec9997932270196996d54d3568af"
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
