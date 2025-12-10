Output urocket-demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1600
Set Height 900
Set Theme "Dracula"

########################################
# 1. Start tmux and configure panes
########################################

Type "tmux"
Enter
Sleep 1s

# Pane layout:
# [ sh-serv | sh-front (top) ]
# [          | sh-back (bottom) ]

########################################
# 2. Split window: vertical left + right
########################################

Hide
# Vertical split (creates left and right panes)
Ctrl+B
Type "%"
Sleep 500ms

# After split, we are in the RIGHT pane
# Split this right pane horizontally
Ctrl+B
Type '"'
Sleep 500ms
Show

########################################
# Label panes
########################################

# Left pane = sh-serv (go left)
Ctrl+B
Left
Sleep 500ms
Type "echo '### sh-serv ###'"
Enter
Sleep 800ms

# Top-right pane = sh-front (go right then up)
Ctrl+B
Right
Sleep 300ms
Ctrl+B
Up
Sleep 500ms
Type "echo '### sh-front ###'"
Enter
Sleep 800ms

# Bottom-right pane = sh-back (go down)
Ctrl+B
Down
Sleep 500ms
Type "echo '### sh-back ###'"
Enter
Sleep 800ms


########################################
# 3. Command in sh-serv
########################################

# Return to left pane (sh-serv)
Ctrl+B
Left
Sleep 500ms

Type "cargo run -- -c examples/urocket-service.yaml"
Enter
Sleep 3s


########################################
# 4. Command in sh-front (blocked)
########################################

Ctrl+B
Right
Sleep 500ms

Ctrl+B
Up
Sleep 500ms

Type "curl -X POST localhost:8080/get/pets -H'Content-type: application/json' -d'{" 
Type '"payload":"low"'
Type "}'"
Enter
Sleep 1s


########################################
# 5. First command in sh-back
########################################

Ctrl+B
Down
Sleep 500ms

Type "curl -X POST --unix-socket /tmp/urocket.sock http://internal/urhttp/something -H'Content-type: application/json' -d'{" 
Type '"hello":true'
Type "}'"
Enter
Sleep 1.5s


########################################
# 6. Second command in sh-back with reqid
########################################

Type "curl -X POST --unix-socket /tmp/urocket.sock http://internal/urhttp/64ed1763-9ca9-4b95-b47f-75c1318b3462 -H'Content-type: application/json' -d'{" 
Type '"hello":true'
Type "}'"
Enter
Sleep 1.5s


########################################
# 7. sh-front unblocked and shows response
########################################

Ctrl+B
Up
Sleep 2s

########################################
# End demo
########################################

Sleep 1s

