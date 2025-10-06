#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/bot"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-bot -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-bot -- bash -c "

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

useradd --system --no-create-home --shell /bin/false gbuser
"

BOT_UID=$(lxc exec "$PARAM_TENANT"-bot -- id -u gbuser)
BOT_GID=$(lxc exec "$PARAM_TENANT"-bot -- id -g gbuser)
HOST_BOT_UID=$((100000 + BOT_UID))
HOST_BOT_GID=$((100000 + BOT_GID))
chown -R "$HOST_BOT_UID:$HOST_BOT_GID" "$HOST_BASE"

lxc config device add "$PARAM_TENANT"-bot botdata disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$PARAM_TENANT"-bot botconf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$PARAM_TENANT"-bot botlogs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-bot -- bash -c '
mkdir -p /opt/gbo/data /opt/gbo/conf /opt/gbo/logs

sudo apt update
sudo apt install -y curl gnupg ca-certificates git

curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash -
sudo apt install -y nodejs

sudo apt install -y xvfb libgbm-dev

wget https://dl.google.com/linux/chrome/deb/pool/main/g/google-chrome-stable/google-chrome-stable_128.0.6613.119-1_amd64.deb
sudo apt install ./google-chrome-stable_128.0.6613.119-1_amd64.deb

cd /opt/gbo/data
git clone https://alm.pragmatismo.com.br/generalbots/botserver.git
cd botserver
npm install

./node_modules/.bin/tsc
cd packages/default.gbui
npm install
npm run build

chown -R gbuser:gbuser /opt/gbo

# Create systemd service
sudo tee /etc/systemd/system/bot.service > /dev/null <<EOF
[Unit]
Description=Bot Server
After=network.target

[Service]
User=gbuser
Group=gbuser
Environment="DISPLAY=:99"
ExecStartPre=/bin/bash -c "/usr/bin/Xvfb :99 -screen 0 1024x768x24 -ac +extension GLX +render -noreset &"
WorkingDirectory=/opt/gbo/data/botserver
ExecStart=/usr/bin/node /opt/gbo/data/botserver/boot.mjs
Restart=always
RestartSec=5
StandardOutput=append:/opt/gbo/logs/stdout.log
StandardError=append:/opt/gbo/logs/stderr.log

[Install]
WantedBy=multi-user.target
EOF

# Reload and start service
sudo systemctl daemon-reload
sudo systemctl enable bot.service
sudo systemctl start bot.service
'

lxc config device remove "$PARAM_TENANT"-bot bot-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-bot bot-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_BOT_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_BOT_PORT"
