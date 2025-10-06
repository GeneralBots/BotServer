#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/meeting"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-meeting -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-meeting -- bash -c "

apt-get update && apt-get install -y wget coturn
mkdir -p /opt/gbo/bin
cd /opt/gbo/bin
wget -q https://github.com/livekit/livekit/releases/download/v1.8.4/livekit_1.8.4_linux_amd64.tar.gz
tar -xzf livekit*.tar.gz
rm livekit_1.8.4_linux_amd64.tar.gz
chmod +x livekit-server

while netstat -tuln | grep -q \":$PARAM_MEETING_TURN_PORT \"; do
  ((PARAM_MEETING_TURN_PORT++))
done

useradd --system --no-create-home --shell /bin/false gbuser

"

MEETING_UID=$(lxc exec "$PARAM_TENANT"-meeting -- id -u gbuser)
MEETING_GID=$(lxc exec "$PARAM_TENANT"-meeting -- id -g gbuser)
HOST_MEETING_UID=$((100000 + MEETING_UID))
HOST_MEETING_GID=$((100000 + MEETING_GID))
chown -R "$HOST_MEETING_UID:$HOST_MEETING_GID" "$HOST_BASE"

lxc config device add "$PARAM_TENANT"-meeting meetingdata disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$PARAM_TENANT"-meeting meetingconf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$PARAM_TENANT"-meeting meetinglogs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-meeting -- bash -c "

mkdir -p /opt/gbo/data /opt/gbo/conf /opt/gbo/logs
chown -R gbuser:gbuser /opt/gbo/data /opt/gbo/conf /opt/gbo/logs

sudo chown gbuser:gbuser /var/run/turnserver.pid


cat > /etc/systemd/system/meeting.service <<EOF
[Unit]
Description=LiveKit Server
After=network.target

[Service]
User=gbuser
Group=gbuser
ExecStart=/opt/gbo/bin/livekit-server --config /opt/gbo/conf/config.yaml
Restart=always
Environment=TURN_PORT=$PARAM_MEETING_TURN_PORT

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/meeting-turn.service <<EOF
[Unit]
Description=TURN Server
After=network.target

[Service]
User=gbuser
Group=gbuser
ExecStart=/usr/bin/turnserver -c /opt/gbo/conf/turnserver.conf
Restart=always

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable meeting meeting-turn
systemctl start meeting meeting-turn
"

lxc config device remove "$PARAM_TENANT"-meeting meeting-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-meeting meeting-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_MEETING_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_MEETING_PORT"