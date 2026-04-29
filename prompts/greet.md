---
start: 
    say: "hi"
success:
    say: "winner, winner, winner, TV dinner"
    message: "{{ctx.now}}: The greet prompt was run "
---
Hi how are you? My name is {{name || "Bob"}}.
