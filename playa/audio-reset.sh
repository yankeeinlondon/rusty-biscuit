#!/usr/bin/env bash

sudo killall coreaudiod
sleep 2
sudo launchctl print system/com.apple.audio.coreaudiod | grep -E 'state =|pid =|last exit code ='
