#!/bin/sh
# demo helper for an executeCommand binding: greet, echoing any arguments and the
# active input value. sextant shows the first stdout line on the base screen, so
# keep it short.
if [ "$#" -gt 0 ]; then
  echo "hi $*"
else
  echo "hi ${AMBIT_VALUE:-there}"
fi
