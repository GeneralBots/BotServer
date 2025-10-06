#!/bin/bash
HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/proxy"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"
mkdir -p "$HOST_BASE" "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod 750 "$HOST_BASE" "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"

lxc launch images:debian/12 "$PARAM_TENANT"-proxy -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-proxy -- bash -c "
mkdir -p /opt/gbo/{bin,data,conf,logs}
apt-get update && apt-get install -y wget libcap2-bin
wget -q https://github.com/caddyserver/caddy/releases/download/v2.10.0-beta.3/caddy_2.10.0-beta.3_linux_amd64.tar.gz
tar -xzf caddy_2.10.0-beta.3_linux_amd64.tar.gz -C /opt/gbo/bin
rm caddy_2.10.0-beta.3_linux_amd64.tar.gz
chmod 750 /opt/gbo/bin/caddy
setcap 'cap_net_bind_service=+ep' /opt/gbo/bin/caddy
useradd --create-home --system --shell /usr/sbin/nologin gbuser
chown -R gbuser:gbuser /opt/gbo/{bin,data,conf,logs}
"

lxc config device add "$PARAM_TENANT"-proxy data disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$PARAM_TENANT"-proxy conf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$PARAM_TENANT"-proxy logs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-proxy -- bash -c "
cat > /etc/systemd/system/proxy.service <<EOF
[Unit]
Description=Proxy
After=network.target
[Service]
User=gbuser
Group=gbuser
Environment=XDG_DATA_HOME=/opt/gbo/data
ExecStart=/opt/gbo/bin/caddy run --config /opt/gbo/conf/config --adapter caddyfile
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload

chown -R gbuser:gbuser /opt/gbo/{bin,data,conf,logs}

systemctl enable proxy
"

    for port in 80 443; do
    lxc config device remove "$PARAM_TENANT"-proxy "port-$port" 2>/dev/null || true
    lxc config device add "$PARAM_TENANT"-proxy "port-$port" proxy listen=tcp:0.0.0.0:$port connect=tcp:127.0.0.1:$port
    done

lxc config set "$PARAM_TENANT"-proxy security.syscalls.intercept.mknod true
lxc config set "$PARAM_TENANT"-proxy security.syscalls.intercept.setxattr true
