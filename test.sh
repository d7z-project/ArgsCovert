#!/usr/bin/env bash
echo "child start"
for (( i = 0; i < 100; i++ )); do
    sleep 1
done
echo "exit" >&2
exit 0
