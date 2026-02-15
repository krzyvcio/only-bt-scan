#!/bin/bash
# Test if .env file loads correctly

cd "$(dirname "$0")"

echo "=== .env File Content ==="
cat .env | grep -E "^TELEGRAM|^SCAN|^WEB|^RUST"

echo ""
echo "=== Checking for problematic characters ==="

# Check for spaces around =
if grep -E '\s+=\s+' .env; then
    echo "WARNING: Found spaces around = sign"
else
    echo "OK: No spaces around = sign"
fi

# Check for trailing spaces
if grep -E '\s+$' .env | grep -E "^TELEGRAM|^SCAN|^WEB"; then
    echo "WARNING: Found trailing spaces in variable values"
else
    echo "OK: No trailing spaces"
fi

# Check for missing quotes if value contains special characters
echo ""
echo "=== Checking TELEGRAM variables ==="
BOT_TOKEN=$(grep "^TELEGRAM_BOT_TOKEN=" .env | cut -d= -f2-)
CHAT_ID=$(grep "^TELEGRAM_CHAT_ID=" .env | cut -d= -f2-)

echo "Bot Token length: ${#BOT_TOKEN}"
echo "Chat ID: $CHAT_ID"

if [ ${#BOT_TOKEN} -gt 20 ]; then
    echo "✓ Bot token appears valid (long enough)"
else
    echo "✗ Bot token might be incomplete"
fi

if [ -z "$CHAT_ID" ]; then
    echo "✗ Chat ID is empty!"
else
    echo "✓ Chat ID is set"
fi
