class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.3/mdmore-v0.1.3-aarch64-apple-darwin.tar.gz"
      sha256 "93fe337527789eefd867300d29f6d616e5cb7822187d7d788830df46ad2baf41"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.3/mdmore-v0.1.3-x86_64-apple-darwin.tar.gz"
      sha256 "e4904fa680a97ef0342fa7177536d5871ca4687be38774e855aa0f0fc548443d"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.3/mdmore-v0.1.3-aarch64-unknown-linux-musl.tar.gz"
      sha256 "eb32f5f911a9c5271f01dd54672d1999d93a9573f27470499c4d120d8c71fc78"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.3/mdmore-v0.1.3-x86_64-unknown-linux-musl.tar.gz"
      sha256 "b8279b02040c1f0ce30f9f0e771f9ff07921f40fee501901ad533c9b7a315847"
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
