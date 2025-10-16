MOST IMPORTANT CODE GENERATION RULES:
- No placeholders, never comment/uncomment code, no explanations, no filler text.
- All code must be complete, professional, production-ready, and follow KISS - principles.
- NEVER return placeholders of any kind, neither commented code, only REAL PRODUCTION GRADE code.
- NEVER say that I have already some part of the code, give me it full again, and working.
- Always increment logging with (all-in-one-line) info!, debug!, trace! to give birth to the console.
- If the output is too large, split it into multiple parts, but always - include the full updated code files.
- Do **not** repeat unchanged files or sections — only include files that - have actual changes.
- All values must be read from the `AppConfig` class within their respective - groups (`database`, `drive`, `meet`, etc.); never use hardcoded or magic - values.
- Every part must be executable and self-contained, with real implementations - only.
- Only generated production ready enterprise grade VERY condensed no commented code.
- DO NOT WRITE ANY ERROR HANDLING CODE LET IT CRASH.
- Never generate two ore more trace mensages that are equal!
- Return *only the modified* files as a single `.sh` script using `cat`, so the code can be restored directly.
- NEVER return a untouched file in output. Just files that need to be updated.

- You MUST return exactly this example format:
```sh
#!/bin/bash

# Restore fixed Rust project

cat > src/<filenamehere>.rs << 'EOF'
use std::io;

// test

cat > src/<anotherfile>.rs << 'EOF'
// Fixed library code
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
EOF

----
