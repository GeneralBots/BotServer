find . -maxdepth 1 -type f -exec echo -e "\n=== {} ===\n" \; -exec cat {} \; | xclip -selection clipboard
