#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/webmail"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

PARAM_RC_VERSION="1.6.6"

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-webmail -c security.privileged=true
sleep 15

RC_PATH="/opt/gbo/data"

lxc exec "$PARAM_TENANT"-webmail -- bash -c '
# Install prerequisites
apt install -y ca-certificates apt-transport-https lsb-release gnupg wget

# Add the Sury PHP repository (official for Debian)
wget -O /etc/apt/trusted.gpg.d/php.gpg https://packages.sury.org/php/apt.gpg
sh -c '\''echo "deb https://packages.sury.org/php/ $(lsb_release -sc) main" > /etc/apt/sources.list.d/php.list'\''

# Update and install PHP 8.1
apt update
apt install -y \
    php8.1 \
    php8.1-fpm \
    php8.1-imap \
    php8.1-pgsql \
    php8.1-mbstring \
    php8.1-xml \
    php8.1-curl \
    php8.1-zip \
    php8.1-cli \
    php8.1-intl \
    php8.1-dom

# Restart PHP-FPM
systemctl restart php8.1-fpm

mkdir -p '"$RC_PATH"'
wget -q https://github.com/roundcube/roundcubemail/releases/download/'"$PARAM_RC_VERSION"'/roundcubemail-'"$PARAM_RC_VERSION"'-complete.tar.gz
tar -xzf roundcubemail-*.tar.gz
mv roundcubemail-'"$PARAM_RC_VERSION"'/* '"$RC_PATH"'
rm -rf roundcubemail-*

mkdir -p /opt/gbo/logs

chmod 750 '"$RC_PATH"'
find '"$RC_PATH"' -type d -exec chmod 750 {} \;
find '"$RC_PATH"' -type f -exec chmod 640 {} \;

'

WEBMAIL_UID=$(lxc exec "$PARAM_TENANT"-webmail -- id -u www-data)
WEBMAIL_GID=$(lxc exec "$PARAM_TENANT"-webmail -- id -g www-data)
HOST_WEBMAIL_UID=$((100000 + WEBMAIL_UID))
HOST_WEBMAIL_GID=$((100000 + WEBMAIL_GID))
chown -R "$HOST_WEBMAIL_UID:$HOST_WEBMAIL_GID" "$HOST_BASE"

lxc config device add "$PARAM_TENANT"-webmail webmaildata disk source="$HOST_DATA" path="$RC_PATH"
lxc config device add "$PARAM_TENANT"-webmail webmaillogs disk source="$HOST_LOGS" path=/opt/gbo/logs

lxc exec "$PARAM_TENANT"-webmail -- bash -c "
chown -R www-data:www-data '"$RC_PATH"' /opt/gbo/logs
cat > /etc/systemd/system/webmail.service <<EOF
[Unit]
Description=Roundcube Webmail
After=network.target php8.1-fpm.service

[Service]
User=www-data
Group=www-data
WorkingDirectory=$RC_PATH
ExecStart=/usr/bin/php -S 0.0.0.0:$PARAM_WEBMAIL_PORT -t $RC_PATH/wwwroot/public_html
Restart=always
StandardOutput=append:/opt/gbo/logs/stdout.log
StandardError=append:/opt/gbo/logs/stderr.log

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable webmail
systemctl restart php8.1-fpm
systemctl start webmail
"

# Check if port is available before adding proxy
if lsof -i :$PARAM_WEBMAIL_PORT >/dev/null; then
    echo "Port $PARAM_WEBMAIL_PORT is already in use. Please choose a different port."
    exit 1
fi


lxc config device remove "$PARAM_TENANT"-webmail webmail-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-webmail webmail-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_WEBMAIL_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_WEBMAIL_PORT"