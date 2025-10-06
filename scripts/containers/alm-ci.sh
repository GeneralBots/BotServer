#!/bin/bash

# Configuration
ALM_CI_NAME="CI"
ALM_CI_LABELS="gbo"
FORGEJO_RUNNER_VERSION="v6.3.1"
FORGEJO_RUNNER_BINARY="forgejo-runner-6.3.1-linux-amd64"
CONTAINER_IMAGE="images:debian/12"

# Paths
HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/alm-ci"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"
BIN_PATH="/opt/gbo/bin"
CONTAINER_NAME="${PARAM_TENANT}-alm-ci"

# Create host directories
mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS" || exit 1
chmod -R 750 "$HOST_BASE" || exit 1

# Launch container
if ! lxc launch "$CONTAINER_IMAGE" "$CONTAINER_NAME" -c security.privileged=true; then
    echo "Failed to launch container"
    exit 1
fi

# Wait for container to be ready
for i in {1..10}; do
    if lxc exec "$CONTAINER_NAME" -- bash -c "true"; then
        break
    fi
    sleep 3
done


# Container setup
lxc exec "$CONTAINER_NAME" -- bash -c "
set -e

useradd --system --no-create-home --shell /bin/false $CONTAINER_NAME

# Update and install dependencies
apt-get update && apt-get install -y wget git || { echo 'Package installation failed'; exit 1; }

sudo apt update
sudo apt install -y curl gnupg ca-certificates git
apt-get update && apt-get install -y \
build-essential cmake git pkg-config libjpeg-dev libtiff-dev \
libpng-dev libavcodec-dev libavformat-dev libswscale-dev \
libv4l-dev libatlas-base-dev gfortran python3-dev cpulimit \
expect libxtst-dev libpng-dev

sudo apt-get install -y libcairo2-dev libpango1.0-dev libgif-dev librsvg2-dev
sudo apt install xvfb -y

sudo apt install -y \
  libnss3 \
  libatk1.0-0 \
  libatk-bridge2.0-0 \
  libcups2 \
  libdrm2 \
  libxkbcommon0 \
  libxcomposite1 \
  libxdamage1 \
  libxfixes3 \
  libxrandr2 \
  libgbm1 \
  libasound2 \
  libpangocairo-1.0-0

export OPENCV4NODEJS_DISABLE_AUTOBUILD=1
export OPENCV_LIB_DIR=/usr/lib/x86_64-linux-gnu

sudo apt install -y curl gnupg ca-certificates git

# Install Node.js 22.x
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash -
sudo apt install -y nodejs
npm  install -g pnpm@latest

# Install rust 1.85
apt-get install -y libssl-dev pkg-config
sudo apt-get install -y \
    apt-transport-https \
    software-properties-common \
    gnupg \
    cmake \
    build-essential \
    clang \
    libclang-dev \
    libz-dev \
    libssl-dev \
    pkg-config

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.85.1 -y
source ~/.cargo/env
rustc --version


# Install Xvfb and other dependencies
sudo apt install -y xvfb libgbm-dev lxd-client

# Create directories
mkdir -p \"$BIN_PATH\" /opt/gbo/data /opt/gbo/conf /opt/gbo/logs || { echo 'Directory creation failed'; exit 1; }

# Download and install forgejo-runner
wget -O \"$BIN_PATH/forgejo-runner\" \"https://code.forgejo.org/forgejo/runner/releases/download/$FORGEJO_RUNNER_VERSION/$FORGEJO_RUNNER_BINARY\" || { echo 'Download failed'; exit 1; }
chmod +x \"$BIN_PATH/forgejo-runner\" || { echo 'chmod failed'; exit 1; }

cd \"$BIN_PATH\"

# Register runner
\"$BIN_PATH/forgejo-runner\" register --no-interactive \\
    --name \"$ALM_CI_NAME\" \\
    --instance \"$PARAM_ALM_CI_INSTANCE\" \\
    --token \"$PARAM_ALM_CI_TOKEN\" \\
    --labels \"$ALM_CI_LABELS\" || { echo 'Runner registration failed'; exit 1; }

chown -R $CONTAINER_NAME:$CONTAINER_NAME /opt/gbo/bin /opt/gbo/data /opt/gbo/conf /opt/gbo/logs

"

# Set permissions
echo "[CONTAINER] Setting permissions..."
EMAIL_UID=$(lxc exec "$PARAM_TENANT"-alm-ci -- id -u $CONTAINER_NAME)
EMAIL_GID=$(lxc exec "$PARAM_TENANT"-alm-ci -- id -g $CONTAINER_NAME)
HOST_EMAIL_UID=$((100000 + EMAIL_UID))
HOST_EMAIL_GID=$((100000 + EMAIL_GID))
sudo chown -R "$HOST_EMAIL_UID:$HOST_EMAIL_GID" "$HOST_BASE"


# Add directory mappings
lxc config device add "$CONTAINER_NAME" almdata disk source="$HOST_DATA" path=/opt/gbo/data || exit 1
lxc config device add "$CONTAINER_NAME" almconf disk source="$HOST_CONF" path=/opt/gbo/conf || exit 1
lxc config device add "$CONTAINER_NAME" almlogs disk source="$HOST_LOGS" path=/opt/gbo/logs || exit 1


lxc exec "$CONTAINER_NAME" -- bash -c "
# Create systemd service
cat > /etc/systemd/system/alm-ci.service <<EOF
[Unit]
Description=ALM CI Runner
After=network.target

[Service]
Type=simple
User=$CONTAINER_NAME
Group=$CONTAINER_NAME
ExecStart=$BIN_PATH/forgejo-runner daemon
Restart=always
RestartSec=5
StandardOutput=append:/opt/gbo/logs/output.log
StandardError=append:/opt/gbo/logs/error.log

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
systemctl daemon-reload || { echo 'daemon-reload failed'; exit 1; }
systemctl enable alm-ci || { echo 'enable service failed'; exit 1; }
systemctl start alm-ci || { echo 'start service failed'; exit 1; }
"


LXC_BOT="/opt/gbo/tenants/$PARAM_TENANT/bot/data"
LXC_PROXY="/opt/gbo/tenants/$PARAM_TENANT/proxy/data/websites"
LXC_SYSTEM="/opt/gbo/tenants/$PARAM_TENANT/system/bin"

lxc config device add "$CONTAINER_NAME" almbot disk source="$LXC_BOT" path=/opt/gbo/bin/bot
lxc config device add "$CONTAINER_NAME" almproxy disk source="$LXC_PROXY" path=/opt/gbo/bin/proxy
lxc config device add "$CONTAINER_NAME" almsystem disk source="$LXC_SYSTEM" path=/opt/gbo/bin/syst  em || exit 1
