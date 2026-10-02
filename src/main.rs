mod pager;

use std::io::{self, IsTerminal, Read, Write};
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use mdmore::render::Renderer;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Read Markdown in the terminal.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Markdown file to read; omit or use - to read standard input.
    file: Option<PathBuf>,
    /// Render to standard output without opening the pager.
    #[arg(short = 'p', long)]
    no_pager: bool,
    /// Render without ANSI styling or syntax highlighting; implies --no-pager.
    #[arg(long)]
    plain: bool,
    /// When to emit ANSI styling (auto respects NO_COLOR and TERM=dumb).
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorMode,
    /// Maximum content width in terminal cells (default: terminal width or 80).
    #[arg(short = 'w', long)]
    width: Option<NonZeroU16>,
    /// Skip syntax highlighting of fenced code blocks.
    #[arg(long)]
    no_highlight: bool,
    /// Disable mouse wheel handling in the pager.
    #[arg(long)]
    no_mouse: bool,
}

fn run(cli: Cli) -> io::Result<()> {
    if cli.file.is_none() && io::stdin().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "specify a Markdown file, or pipe Markdown into mdmore (see --help)",
        ));
    }
    let from_stdin = cli.file.as_ref().is_none_or(|path| path.as_os_str() == "-");
    let mut source = String::new();
    let name = if from_stdin {
        io::stdin().read_to_string(&mut source)?;
        "stdin".to_owned()
    } else {
        let path = cli.file.as_ref().expect("file supplied");
        source = std::fs::read_to_string(path).map_err(|error| {
            io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;
        path.display().to_string()
    };
    let dumb = std::env::var_os("TERM").is_some_and(|term| term == "dumb");
    let color = !cli.plain
        && match cli.color {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                io::stdout().is_terminal() && !dumb && std::env::var_os("NO_COLOR").is_none()
            }
        };
    let highlight = color && !cli.no_highlight;
    // Override Crossterm's own NO_COLOR handling after resolving --color.
    crossterm::style::force_color_output(color);
    let terminal_width = crossterm::terminal::size().map_or(80, |(width, _)| width.max(1));
    let width = cli.width.map_or(terminal_width, NonZeroU16::get);
    let interactive = !cli.no_pager
        && !cli.plain
        && !dumb
        && io::stdout().is_terminal()
        && has_controlling_terminal();
    if interactive {
        connect_keyboard()?;
        pager::run(
            &source,
            &name,
            cli.width.map(NonZeroU16::get),
            color,
            highlight,
            !cli.no_mouse,
        )
    } else {
        let mut out = io::BufWriter::new(io::stdout().lock());
        for line in Renderer::new(&source, usize::from(width), highlight) {
            line.write(&mut out, color, "")?;
            out.write_all(b"\n")?;
        }
        out.flush()
    }
}

#[cfg(unix)]
fn has_controlling_terminal() -> bool {
    std::fs::File::open("/dev/tty").is_ok()
}

#[cfg(not(unix))]
fn has_controlling_terminal() -> bool {
    io::stdin().is_terminal()
}

#[cfg(unix)]
fn connect_keyboard() -> io::Result<()> {
    if !io::stdin().is_terminal() {
        use std::os::unix::ffi::OsStrExt;
        // After reading the pipe, use a terminal for keyboard input. Open the
        // device itself: macOS kqueue cannot poll the /dev/tty alias.
        let name = rustix::termios::ttyname(io::stdout(), Vec::new())?;
        let tty = std::fs::File::open(std::ffi::OsStr::from_bytes(name.to_bytes()))?;
        rustix::stdio::dup2_stdin(&tty)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn connect_keyboard() -> io::Result<()> {
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mdmore: {error}");
            ExitCode::FAILURE
        }
    }
}
