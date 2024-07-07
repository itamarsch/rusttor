#!/bin/bash

# Create a new window
tmux new-window -n 'Tor'
# Split the window into a 4x2 grid
tmux split-window -h
tmux split-window -h
tmux select-pane -t 1
tmux split-window -h

tmux select-pane -t 1
tmux split-window -v
tmux select-pane -t 3
tmux split-window -v
tmux select-pane -t 5
tmux split-window -v

tmux select-pane -t 7
tmux split-window -v
LOG=debug
for i in {1..6}; do
	port=$((10000 + i - 1))
	tmux select-pane -t $i
	tmux send-keys "clear && RUST_LOG=$LOG cargo run -q --bin node -- -p $port" C-m
done

sleep 2
tmux send-keys -t 7 "clear && RUST_LOG=$LOG cargo run -q --bin client" C-m
tmux send-keys -t 8 "python3 ./src/tor/node/test_server.py 12345" C-m
