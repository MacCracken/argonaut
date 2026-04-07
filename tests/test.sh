#!/bin/sh
CC="${1:-./build/cc2}"
echo "=== argonaut tests ==="
cat src/main.cyr | "$CC" > /tmp/argonaut_test && chmod +x /tmp/argonaut_test && /tmp/argonaut_test
echo "exit: $?"
rm -f /tmp/argonaut_test
