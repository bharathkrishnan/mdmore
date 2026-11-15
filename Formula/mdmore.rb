class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.5/mdmore-v0.1.5-aarch64-apple-darwin.tar.gz"
      sha256 "6d5c7c4f189c6b35b3e1e00f106e92071d420b14e9bed7ca207a49b9454b670b"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.5/mdmore-v0.1.5-x86_64-apple-darwin.tar.gz"
      sha256 "037242004fb9b21c1fdc7b010d3f71b4fb21ca7a67188962aa6300fafcfc5989"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.5/mdmore-v0.1.5-aarch64-unknown-linux-musl.tar.gz"
      sha256 "72eb531334daa8e64cd03ebf56387f1cb0c1a575f2e65b850b3b4685ca353d6a"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.5/mdmore-v0.1.5-x86_64-unknown-linux-musl.tar.gz"
      sha256 "d6d90244faec00ff887128a630ee76707fdc090e50bab360a0141f4f2579793e"
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
