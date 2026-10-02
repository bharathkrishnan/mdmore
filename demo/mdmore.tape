Require mdmore

Output mdmore.gif
Output mdmore.mp4

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 20
Set Width 1280
Set Height 720
Set Padding 20
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Framerate 30
Set TypingSpeed 60ms
Set WaitTimeout 8s

Hide
Type "unset NO_COLOR; export PS1='$ ' COLORTERM=truecolor; clear"
Enter
Sleep 500ms
Show

Type "mdmore ripgrep.md"
Enter
Wait /Space:page/
Sleep 2s

# Page through the comparison tables, then go back one page.
Space
Sleep 1s
Down@30ms 20
Wait+Screen /Tool.*Command/
Sleep 3s
Space
Sleep 2s
Type "b"
Sleep 1s

# Search forward to the installation instructions.
Type "/Installation"
Enter
Wait+Screen /Archives of precompiled binaries/
Sleep 2s
Escape
Space
Sleep 2s

# Jump to the build instructions and their code blocks.
Type "/Building"
Enter
Wait+Screen /cargo build/
Sleep 2s
Escape
Down 4
Sleep 2s

Type "?"
Wait+Screen /mdmore — keys/
Sleep 2s
Escape
Type "g"
Sleep 1s
Type "q"
Sleep 1s
