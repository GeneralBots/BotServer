#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
OUTPUT_FILE="$SCRIPT_DIR/prompt.out"

rm -f "$OUTPUT_FILE"
echo "Consolidated LLM Context" > "$OUTPUT_FILE"

prompts=(
    "./prompts/dev/shared.md"
    "./Cargo.toml"
    "./prompts/dev/generation.md"
)

for file in "${prompts[@]}"; do
    if [ -f "$file" ]; then
        cat "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi
done

dirs=(
    #"auth"
    #"automation"
    #"basic"
    "bot"
    "channels"
    #"config"
    #"context"
    #"email"
    #"file"
    #"llm"
    #"llm_legacy"
    #"org"
    "session"
    "shared"
    #"tests"
    #"tools"
    #"web_automation"
    #"whatsapp"
)

filter_rust_file() {
    sed -E '/^\s*\/\//d' "$1" | \
    sed -E '/info!\s*\(/d' | \
    sed -E '/debug!\s*\(/d' | \
    sed -E '/trace!\s*\(/d'
}

for dir in "${dirs[@]}"; do
    find "$PROJECT_ROOT/src/$dir" -name "*.rs" | while read -r file; do
        echo "$file" >> "$OUTPUT_FILE"
        filter_rust_file "$file" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    done
done

# Additional specific files
files=(
    "$PROJECT_ROOT/src/main.rs"
    "$PROJECT_ROOT/src/basic/keywords/hear_talk.rs"
    "$PROJECT_ROOT/templates/annoucements.gbai/annoucements.gbdialog/start.bas"
    "$PROJECT_ROOT/templates/annoucements.gbai/annoucements.gbdialog/auth.bas"
    "$PROJECT_ROOT/web/index.html"
)

for file in "${files[@]}"; do
    if [[ "$file" == *.rs ]]; then
        echo "$file" >> "$OUTPUT_FILE"
        filter_rust_file "$file" >> "$OUTPUT_FILE"
    else
        echo "$file" >> "$OUTPUT_FILE"
        cat "$file" >> "$OUTPUT_FILE"
    fi
done

# Remove all blank lines and reduce whitespace greater than 1 space
sed -i 's/[[:space:]]*$//' "$OUTPUT_FILE"
sed -i '/^$/d' "$OUTPUT_FILE"
sed -i 's/  \+/ /g' "$OUTPUT_FILE"

# Calculate and display token count (approximation: words * 1.3)
WORD_COUNT=$(wc -w < "$OUTPUT_FILE")
TOKEN_COUNT=$(echo "$WORD_COUNT * 1.3 / 1" | bc)
FILE_SIZE=$(wc -c < "$OUTPUT_FILE")

echo "" >> "$OUTPUT_FILE"

echo "Approximate token count: $TOKEN_COUNT"
echo "Context size: $FILE_SIZE bytes"

cat "$OUTPUT_FILE" | xclip -selection clipboard
echo "Content copied to clipboard (xclip)"
