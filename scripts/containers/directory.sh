#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/directory"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

sudo mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
sudo chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-directory -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-directory -- bash -c "
apt-get update && apt-get install -y wget libcap2-bin
wget -c https://github.com/zitadel/zitadel/releases/download/v2.71.2/zitadel-linux-amd64.tar.gz -O - | tar -xz -C /tmp
mkdir -p /opt/gbo/bin
mv /tmp/zitadel-linux-amd64/zitadel /opt/gbo/bin/zitadel
chmod +x /opt/gbo/bin/zitadel
sudo setcap 'cap_net_bind_service=+ep' /opt/gbo/bin/zitadel

useradd --system --no-create-home --shell /bin/false gbuser
mkdir -p /opt/gbo/data /opt/gbo/conf /opt/gbo/logs
chown -R gbuser:gbuser /opt/gbo/data /opt/gbo/conf /opt/gbo/logs /opt/gbo/bin
"

GBUSER_UID=$(lxc exec "$PARAM_TENANT"-directory -- id -u gbuser)
GBUSER_GID=$(lxc exec "$PARAM_TENANT"-directory -- id -g gbuser)
HOST_GBUSER_UID=$((100000 + GBUSER_UID))
HOST_GBUSER_GID=$((100000 + GBUSER_GID))
sudo chown -R "$HOST_GBUSER_UID:$HOST_GBUSER_GID" "$HOST_BASE"

lxc config device add "$PARAM_TENANT"-directory directorydata disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$PARAM_TENANT"-directory directoryconf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$PARAM_TENANT"-directory directorylogs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-directory -- bash -c "
chown -R gbuser:gbuser /opt/gbo/data /opt/gbo/conf /opt/gbo/logs /opt/gbo/bin

cat > /etc/systemd/system/directory.service <<EOF
[Unit]
Description=Directory Service
After=network.target

[Service]
Type=simple
User=gbuser
Group=gbuser
ExecStart=/opt/gbo/bin/zitadel start --masterkey $PARAM_DIRECTORY_MASTERKEY --config /opt/gbo/conf/config.yaml --tlsMode external
WorkingDirectory=/opt/gbo/bin
StandardOutput=append:/opt/gbo/logs/output.log
StandardError=append:/opt/gbo/logs/error.log
Restart=always

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable directory
systemctl start directory
"

lxc config device remove "$PARAM_TENANT"-directory directory-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-directory directory-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_DIRECTORY_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_DIRECTORY_PORT"
