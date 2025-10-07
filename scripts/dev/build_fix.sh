#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/prompt.txt"

echo "Consolidated LLM Context" > "$OUTPUT_FILE"

prompts=(
    "../../prompts/dev/general.md"
    "../../Cargo.toml"
    "../../prompts/dev/fix.md"
)

for file in "${prompts[@]}"; do
    cat "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

dirs=(
    "auth"
    "automation"
    "basic"
    "bot"
    "channels"
    "chart"
    "config"
    "context"
    "email"
    "file"
    "llm"
    "llm_legacy"
    "org"
    "session"
    "shared"
    "tests"
    "tools"
    "web_automation"
    "whatsapp"
)

for dir in "${dirs[@]}"; do
    find "$PROJECT_ROOT/src/$dir" -name "*.rs" | while read file; do
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
done

cat "$PROJECT_ROOT/src/main.rs" >> "$OUTPUT_FILE"
echo "" >> "$OUTPUT_FILE"


cd "$PROJECT_ROOT"
tree -P '*.rs' -I 'target|*.lock' --prune | grep -v '[0-9] directories$' >> "$OUTPUT_FILE"


cargo build 2>> "$OUTPUT_FILE"
