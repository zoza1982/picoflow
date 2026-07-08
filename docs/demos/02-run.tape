Output "02-run.gif"
Set Shell "bash"
Set FontSize 16
Set Width 1400
Set Height 980
Set Padding 22
Set Theme "Catppuccin Mocha"
Set TypingSpeed 78ms
Hide
Type `export PS1="picoflow ❯ "; export PATH="$PWD/bin:$PATH"; rm -f picoflow.db; clear`
Enter
Show
Sleep 1s
Type "picoflow --log-format pretty run pipeline.yaml"
Enter
Sleep 2s
