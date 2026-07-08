Output "03-inspect.gif"
Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 900
Set Padding 22
Set Theme "Catppuccin Mocha"
Set TypingSpeed 42ms
Hide
Type `export PS1="picoflow ❯ "; export PATH="$PWD/bin:$PATH"; rm -f picoflow.db; picoflow run pipeline.yaml >/dev/null 2>&1; clear`
Enter
Show
Sleep 500ms
Type "picoflow --log-level error stats data-pipeline"
Enter
Sleep 3s
Type "picoflow --log-level error status data-pipeline --limit 1"
Enter
Sleep 4s
