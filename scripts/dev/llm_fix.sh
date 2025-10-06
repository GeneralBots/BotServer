#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/llm_context.txt"

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
   "src/channels"
   "src/llm"
   "src/whatsapp"
   "src/config"
    "src/auth"
    "src/shared"
    "src/bot"
    "src/session"
    "src/tools"
    "src/context"
)

for dir in "${dirs[@]}"; do
    find "$PROJECT_ROOT/$dir" -name "*.rs" | while read file; do
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
done

cd "$PROJECT_ROOT"
tree -P '*.rs' -I 'target|*.lock' --prune | grep -v '[0-9] directories$' >> "$OUTPUT_FILE"


cargo build 2>> "$OUTPUT_FILE"
