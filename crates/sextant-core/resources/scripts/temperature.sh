#!/bin/sh
# demo helper for an executeCommand binding: print a temperature for sextant to
# show on the base screen. reads the first thermal zone when available, else a
# fixed placeholder so the demo works on any system.
zone=/sys/class/thermal/thermal_zone0/temp
if [ -r "$zone" ]; then
  echo "$(($(cat "$zone") / 1000))C"
else
  echo "65C"
fi
