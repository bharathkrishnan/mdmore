use std::io::Write;
use std::process::{Command, Stdio};

fn stdin_run(args: &[&str], source: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdmore"))
        .args(args)
        .env_remove("NO_COLOR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(source.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn redirected_output_is_rendered_and_has_no_ansi() {
    let output = stdin_run(&[], "# Heading\n\n**hello** `world`\n");
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("hello world"));
    assert!(!text.contains('\x1b'));
}

#[test]
fn color_is_explicitly_available_to_external_pagers() {
    let output = stdin_run(&["--color", "always", "--no-pager"], "# Heading\n");
    assert!(output.status.success());
    assert!(output.stdout.contains(&0x1b));
}

#[test]
fn plain_overrides_forced_colors_and_accepts_dash() {
    let output = stdin_run(&["--plain", "--color", "always", "-"], "**hello**");
    assert!(output.status.success());
    assert!(!output.stdout.contains(&0x1b));
}

#[test]
fn invalid_width_and_missing_files_fail_clearly() {
    let output = stdin_run(&["--width", "0"], "");
    assert!(!output.status.success());
    let output = Command::new(env!("CARGO_BIN_EXE_mdmore"))
        .arg("/nonexistent/mdmore-test-file.md")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("mdmore-test-file.md")
    );
}

#[test]
fn invalid_utf8_is_rejected() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdmore"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&[0xff, 0xfe])
        .unwrap();
    assert!(!child.wait_with_output().unwrap().status.success());
}

#[test]
fn explicit_color_always_overrides_no_color_environment() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdmore"))
        .args(["--color", "always", "--no-pager"])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"# Heading").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let ansi = String::from_utf8(output.stdout).unwrap();
    assert!(ansi.contains("38;5;14"), "{ansi:?}");
}
