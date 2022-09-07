#!/usr/bin/env bash
echo hello

for (( i = 0; i < 10; i++ )); do
    echo "foreach index $i"
    echo "error index $i" >&2
    sleep 1
done
echo "error" >&2
exit 1
