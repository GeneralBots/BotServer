
wget https://github.com/ggml-org/llama.cpp/releases/download/b6148/llama-b6148-bin-ubuntu-x64.zip

wget https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_0.gguf?download=true
wget https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf
# Phi-3.5-mini-instruct-IQ2_M.gguf

# ./llama-cli -m tinyllama-1.1b-chat-v1.0.Q4_0.gguf     --reasoning-budget 0 --reasoning-format none -mli
# ./llama-cli -m DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf     --system-prompt "<think> </think>Output as JSON: Name 3 colors and their HEX codes. Use format: [{\"name\": \"red\", \"hex\": \"#FF0000\"}]" --reasoning-budget 0 --reasoning-format none -mli
