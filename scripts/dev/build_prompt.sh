#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/prompt.out"
rm $OUTPUT_FILE
echo "Consolidated LLM Context" > "$OUTPUT_FILE"

prompts=(
    "../../prompts/dev/shared.md"
    "../../Cargo.toml"
    #"../../prompts/dev/fix.md"
    "../../prompts/dev/generation.md"
)

for file in "${prompts[@]}"; do
    cat "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done

dirs=(
    #"auth"
    #"automation"
    #"basic"
    "bot"
    "channels"
    "config"
    "context"
    #"email"
    #"file"
    "llm"
    #"llm_legacy"
    #"org"
    "session"
    "shared"
    #"tests"
    #"tools"
    #"web_automation"
    "whatsapp"
)
for dir in "${dirs[@]}"; do
    find "$PROJECT_ROOT/src/$dir" -name "*.rs" | while read file; do
        echo $file >> "$OUTPUT_FILE"
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
done

# Also append the specific files you mentioned
echo "$PROJECT_ROOT/src/main.rs" >> "$OUTPUT_FILE"
cat "$PROJECT_ROOT/src/main.rs" >> "$OUTPUT_FILE"

cat "$PROJECT_ROOT/src/basic/keywords/hear_talk.rs" >> "$OUTPUT_FILE"
echo "$PROJECT_ROOT/src/basic/mod.rs">> "$OUTPUT_FILE"
cat "$PROJECT_ROOT/src/basic/mod.rs" >> "$OUTPUT_FILE"


echo "" >> "$OUTPUT_FILE"

# cargo build --message-format=short 2>&1 | grep -E 'error' >> "$OUTPUT_FILE"
