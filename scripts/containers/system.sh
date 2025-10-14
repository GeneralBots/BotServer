#!/bin/bash
HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/system"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"
HOST_BIN="$HOST_BASE/bin"
BIN_PATH="/opt/gbo/bin"
CONTAINER_NAME="${PARAM_TENANT}-system"

# Create host directories
mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS" || exit 1
chmod -R 750 "$HOST_BASE" || exit 1


lxc launch images:debian/12 $CONTAINER_NAME -c security.privileged=true
sleep 15

lxc exec $CONTAINER_NAME -- bash -c '

apt-get update && apt-get install -y wget curl unzip git


useradd -r -s /bin/false gbuser || true
mkdir -p /opt/gbo/logs /opt/gbo/bin /opt/gbo/data /opt/gbo/conf
chown -R gbuser:gbuser /opt/gbo/

wget https://github.com/ggml-org/llama.cpp/releases/download/b6148/llama-b6148-bin-ubuntu-x64.zip
mkdir llm
mv llama-b6148-bin-ubuntu-x64.zip llm
cd llm
unzip llama-b6148-bin-ubuntu-x64.zip
mv build/bin/* .
rm build/bin -r
rm llama-b6148-bin-ubuntu-x64.zip

sudo apt install lib-pq
wget https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf
wget https://huggingface.co/CompendiumLabs/bge-small-en-v1.5-gguf/resolve/main/bge-small-en-v1.5-f32.gguf

sudo curl -fsSLo /usr/share/keyrings/brave-browser-beta-archive-keyring.gpg https://brave-browser-apt-beta.s3.brave.com/brave-browser-beta-archive-keyring.gpg
sudo curl -fsSLo /etc/apt/sources.list.d/brave-browser-beta.sources https://brave-browser-apt-beta.s3.brave.com/brave-browser.sources
sudo apt update

sudo apt install brave-browser-beta

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
git clone https://alm.pragmatismo.com.br/generalbots/gbserver

apt install -y build-essential \
    pkg-config \
    libssl-dev \
    gcc-multilib \
    g++-multilib \
    clang \
    lld \
    binutils-dev \
    libudev-dev \
    libdbus-1-dev


cat > /etc/systemd/system/system.service <<EOF
[Unit]
Description=General Bots System Service
After=network.target

[Service]
Type=simple
User=gbuser
Group=gbuser
ExecStart=/opt/gbo/bin/gbserver
StandardOutput=append:/opt/gbo/logs/output.log
StandardError=append:/opt/gbo/logs/error.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable system
systemctl start system
'

lxc config device add $CONTAINER_NAME bin disk source="${HOST_BIN}" path=/opt/gbo/bin
lxc config device add $CONTAINER_NAME data disk source="${HOST_DATA}" path=/opt/gbo/data
lxc config device add $CONTAINER_NAME conf disk source="${HOST_CONF}" path=/opt/gbo/conf
lxc config device add $CONTAINER_NAME logs disk source="${HOST_LOGS}" path=/opt/gbo/logs
lxc config device add $CONTAINER_NAME system-proxy disk source="/opt/gbo/tenants/$PARAM_TENANT/proxy" path=/opt/gbo/refs/proxy


lxc config device remove $CONTAINER_NAME proxy 2>/dev/null || true
lxc config device add $CONTAINER_NAME proxy proxy \
    listen=tcp:0.0.0.0:"${PARAM_SYSTEM_PORT}" \
    connect=tcp:127.0.0.1:"${PARAM_SYSTEM_PORT}"
