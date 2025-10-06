#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/alm"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"
ALM_PATH=/opt/gbo/bin

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-alm -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-alm -- bash -c "
apt-get update && apt-get install -y git git-lfs wget
mkdir -p /opt/gbo/bin
wget https://codeberg.org/forgejo/forgejo/releases/download/v10.0.2/forgejo-10.0.2-linux-amd64 -O $ALM_PATH/forgejo
chmod +x $ALM_PATH/forgejo
useradd --system --no-create-home --shell /bin/false alm
"

FORGEJO_UID=$(lxc exec "$PARAM_TENANT"-alm -- id -u alm)
FORGEJO_GID=$(lxc exec "$PARAM_TENANT"-alm -- id -g alm)
HOST_FORGEJO_UID=$((100000 + FORGEJO_UID))
HOST_FORGEJO_GID=$((100000 + FORGEJO_GID))
chown -R "$HOST_FORGEJO_UID:$HOST_FORGEJO_GID" "$HOST_BASE"

lxc config device add "$PARAM_TENANT"-alm almdata disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$PARAM_TENANT"-alm almconf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$PARAM_TENANT"-alm almlogs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-alm -- bash -c "
mkdir -p /opt/gbo/data /opt/gbo/conf /opt/gbo/logs
chown -R alm:alm /opt/gbo


cat > /etc/systemd/system/alm.service <<EOF
[Unit]
Description=alm
After=network.target

[Service]
User=alm
Group=alm
WorkingDirectory=/opt/gbo/data
ExecStart=/opt/gbo/bin/forgejo web --config /opt/gbo/conf/app.ini
Restart=always
Environment=USER=alm HOME=/opt/gbo/data
StandardOutput=append:/opt/gbo/logs/stdout.log
StandardError=append:/opt/gbo/logs/stderr.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable alm
systemctl start alm
"

lxc config device remove "$PARAM_TENANT"-alm alm-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-alm alm-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_ALM_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_ALM_PORT"