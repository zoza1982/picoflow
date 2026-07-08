Output "01-quickstart.gif"
Set Shell "bash"
Set FontSize 18
Set Width 1400
Set Height 840
Set Padding 22
Set Theme "Catppuccin Mocha"
Set TypingSpeed 42ms
Hide
Type `export PS1="picoflow ❯ "; export PATH="$PWD/bin:$PATH"; rm -f picoflow.db; clear`
Enter
Show
Sleep 500ms
Type "picoflow --version"
Enter
Sleep 900ms
Type@26ms "cat pipeline.yaml"
Enter
Sleep 2.6s
Type "picoflow --log-level error validate pipeline.yaml"
Enter
Sleep 3.5s
