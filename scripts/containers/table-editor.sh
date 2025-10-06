    #!/bin/bash

# Fixed container name
CONTAINER_NAME="$PARAM_TENANT-table-editor"

TABLE_EDITOR_PORT="5757"

# Paths
HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/table-editor"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"
BIN_PATH="/opt/gbo/bin"

# Create host directories
mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

# Launch container
lxc launch images:debian/12 "$CONTAINER_NAME" -c security.privileged=true

# Wait for container to be ready
sleep 10

# Container setup
lxc exec "$CONTAINER_NAME" -- bash -c "
useradd --system --no-create-home --shell /bin/false gbuser
apt-get update
apt-get install -y wget curl

# Create directories
mkdir -p \"$BIN_PATH\" /opt/gbo/data /opt/gbo/conf /opt/gbo/logs

# Download and install NocoDB binary
cd \"$BIN_PATH\"
curl http://get.nocodb.com/linux-x64 -o nocodb -L
chmod +x nocodb
"

# Set permissions
TE_UID=$(lxc exec "$CONTAINER_NAME" -- id -u gbuser)
TE_GID=$(lxc exec "$CONTAINER_NAME" -- id -g gbuser)
HOST_TE_UID=$((100000 + TE_UID))
HOST_TE_GID=$((100000 + TE_GID))
chown -R "$HOST_TE_UID:$HOST_TE_GID" "$HOST_BASE"

# Add directory mappings
lxc config device add "$CONTAINER_NAME" tedata disk source="$HOST_DATA" path=/opt/gbo/data
lxc config device add "$CONTAINER_NAME" teconf disk source="$HOST_CONF" path=/opt/gbo/conf
lxc config device add "$CONTAINER_NAME" telogs disk source="$HOST_LOGS" path=/opt/gbo/logs




# Create systemd service
lxc exec "$CONTAINER_NAME" -- bash -c "
cat > /etc/systemd/system/table-editor.service <<EOF
[Unit]
Description=NocoDB Table Editor
After=network.target

[Service]
Type=simple
User=gbuser
Group=gbuser
WorkingDirectory=$BIN_PATH
Environment=PORT=${PARAM_TABLE_EDITOR_PORT}
Environment=DATABASE_URL=postgres://${PARAM_TABLES_USER}:${PARAM_TABLES_PASSWORD}@${PARAM_TABLES_HOST}:${PARAM_TABLES_PORT}/${PARAM_TABLE_EDITOR_DATABASE}
ExecStart=$BIN_PATH/nocodb
Restart=always
StandardOutput=append:/opt/gbo/logs/out.log
StandardError=append:/opt/gbo/logs/err.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable table-editor
systemctl start table-editor
"



# Expose the NocoDB port
lxc config device add "$CONTAINER_NAME" http proxy listen=tcp:0.0.0.0:$TABLE_EDITOR_PORT connect=tcp:127.0.0.1:$TABLE_EDITOR_PORT