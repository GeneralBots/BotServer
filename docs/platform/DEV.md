# LLM

ZED for Windows: https://zed.dev/windows

Zed Assistant: Groq + GPT OSS 120B |
FIX Manual: DeepSeek | ChatGPT 120B | Claude 4.5 Thinking | Mistral
ADD Manual: Claude/DeepSeek -> DeepSeek

# Install


cargo install cargo-audit
cargo install cargo-edit
apt install -y libpq-dev
apt install -y valkey-cli

## Cache

curl -fsSL https://packages.redis.io/gpg | sudo gpg --dearmor -o /usr/share/keyrings/valkey.gpg
echo "deb [signed-by=/usr/share/keyrings/valkey.gpg] https://packages.redis.io/deb $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/valkey.list
sudo apt install valkey-server

## Meet

curl -sSL https://get.livekit.io | bash
livekit-server --dev



# Util

cargo upgrade
cargo audit

valkey-cli -p 6379 monitor

# Prompt add-ons

- Prompt add-ons: Fill the file with info!, trace! and debug! macros.
-
