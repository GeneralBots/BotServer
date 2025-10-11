#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/prompt.out"
rm $OUTPUT_FILE
echo "Consolidated LLM Context" > "$OUTPUT_FILE"

prompts=(
    "../../prompts/dev/general.md"
    "../../Cargo.toml"
    # "../../prompts/dev/fix.md"
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
    #"channels"
    "config"
    "context"
    #"email"
    #"file"
    "llm"
    #"llm_legacy"
    #"org"
    #"session"
    "shared"
    #"tests"
    #"tools"
    #"web_automation"
    #"whatsapp"
)

for dir in "${dirs[@]}"; do
    find "$PROJECT_ROOT/src/$dir" -name "*.rs" | while read file; do
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
done

cat "$PROJECT_ROOT/src/main.rs" >> "$OUTPUT_FILE"
cat "$PROJECT_ROOT/src/basic/keywords/hear_talk.rs" >> "$OUTPUT_FILE"
cat "$PROJECT_ROOT/templates/annoucements.gbai/annoucements.gbdialog/start.bas" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"


cd "$PROJECT_ROOT"
find "$PROJECT_ROOT/src" -type f -name "*.rs" ! -path "*/target/*" ! -name "*.lock" -print0 |
while IFS= read -r -d '' file; do
    echo "File: ${file#$PROJECT_ROOT/}" >> "$OUTPUT_FILE"
    grep -E '^\s*(pub\s+)?(fn|struct)\s' "$file" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"
done


# cargo build 2>> "$OUTPUT_FILE"
