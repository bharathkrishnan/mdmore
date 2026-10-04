class Mdmore < Formula
  desc "Markdown pager with progressive rendering"
  homepage "https://github.com/bharathkrishnan/mdmore"
  license "MIT"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.1/mdmore-v0.1.1-aarch64-apple-darwin.tar.gz"
      sha256 "e640ebba4bf4e44db26ff3a2407629e19735d61b7c758bdf6e0a286ca9d1b361"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.1/mdmore-v0.1.1-x86_64-apple-darwin.tar.gz"
      sha256 "091e55419c44419664f663c77b09c6c187eac855389e79eb781cb0a11dfde542"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.1/mdmore-v0.1.1-aarch64-unknown-linux-musl.tar.gz"
      sha256 "a0ab8edc7d9a193b59d93eed7aab9696b124a678b0122feed3c1c937855d142a"
    end
    on_intel do
      url "https://github.com/bharathkrishnan/mdmore/releases/download/v0.1.1/mdmore-v0.1.1-x86_64-unknown-linux-musl.tar.gz"
      sha256 "8d063be300c47ab8cc3b725be871d396c60c2fcc9b554b367b27e1d5701002de"
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
