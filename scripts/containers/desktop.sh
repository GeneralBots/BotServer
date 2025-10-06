#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/desktop"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"
chmod -R 750 "$HOST_BASE"

lxc launch images:debian/12 "$PARAM_TENANT"-desktop -c security.privileged=true
sleep 15

lxc exec "$PARAM_TENANT"-desktop -- bash -c "

apt-get update
apt-get install -y xvfb xrdp xfce4 xfce4-goodies
cat > /etc/xrdp/startwm.sh <<EOF
#!/bin/sh
if [ -r /etc/default/locale ]; then
  . /etc/default/locale
  export LANG LANGUAGE
fi
startxfce4
EOF
chmod +x /etc/xrdp/startwm.sh
systemctl restart xrdp
systemctl enable xrdp

# For the root user (since you're logging in as root)
echo "exec startxfce4" > /root/.xsession
chmod +x /root/.xsession

apt install -y curl apt-transport-https gnupg
curl -s https://brave-browser-apt-release.s3.brave.com/brave-core.asc | gpg --dearmor > /usr/share/keyrings/brave-browser-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/brave-browser-archive-keyring.gpg] https://brave-browser-apt-release.s3.brave.com/ stable main" > /etc/apt/sources.list.d/brave-browser-release.list
apt update && apt install -y brave-browser

sudo apt install gnome-tweaks
/etc/environment
    GTK_IM_MODULE=cedilla
    QT_IM_MODULE=cedilla

"
port=3389
lxc config device remove "$PARAM_TENANT"-desktop "port-$port" 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-desktop "port-$port" proxy listen=tcp:0.0.0.0:$port connect=tcp:127.0.0.1:$port
