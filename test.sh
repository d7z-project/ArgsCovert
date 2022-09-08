#!/usr/bin/env bash
echo "child start"
for (( i = 0; i < 2; i++ )); do
    echo "foreach :$i"
    sleep 1
done
echo "exit" >&2
exit 0
