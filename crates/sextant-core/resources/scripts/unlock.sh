#!/bin/sh
# demo helper for an executeCommand binding: stand in for a lock integration. it
# prints what it would do and deliberately performs no real action. arg 1 is the
# subcommand ("open" or "set"); "set" reports the active input value.
case "${1:-open}" in
  set) echo "set ${AMBIT_VALUE:-0}" ;;
  *) echo "$* ok" ;;
esac
