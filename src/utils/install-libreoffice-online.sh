sudo apt install -y cloud-guest-utils e2fsprogs

apt install -y make g++ build-essential
apt install -y openjdk-17-jdk ant
apt install -y sudo systemd wget zip procps ccache
apt install -y automake bison flex git gperf graphviz junit4 libtool m4 nasm
apt install -y libcairo2-dev libjpeg-dev libegl1-mesa-dev libfontconfig1-dev \
    libgl1-mesa-dev libgif-dev libgtk-3-dev librsvg2-dev libpango1.0-dev
apt install -y libcap-dev libcap2-bin libkrb5-dev libpcap0.8-dev openssl libssl-dev
apt install -y libxcb-dev libx11-xcb-dev libxkbcommon-x11-dev libxtst-dev \
    libxrender-dev libxslt1-dev libxt-dev xsltproc
apt install -y libcunit1-dev libcppunit-dev libpam0g-dev libcups2-dev libzstd-dev uuid-runtime
apt install -y python3-dev python3-lxml python3-pip python3-polib
apt install -y nodejs npm
apt install -y libpoco-dev libpococrypto80
apt install -y libreoffice-dev


mkdir -p /opt/lo && cd /opt/lo
wget https://github.com/CollaboraOnline/online/releases/download/for-code-assets/core-co-24.04-assets.tar.gz
tar xf core-co-24.04-assets.tar.gz && rm core-co-24.04-assets.tar.gz

useradd cool -G sudo
mkdir -p /opt/cool && chown cool:cool /opt/cool
cd /opt/cool
sudo -Hu cool git clone https://github.com/CollaboraOnline/online.git
cd online && sudo -Hu cool ./autogen.sh

export CPPFLAGS=-I/opt/lo/include
export LDFLAGS=-L/opt/lo/instdir/program
./configure --with-lokit-path=/opt/lo --with-lo-path=/opt/lo/instdir --with-poco-includes=/usr/local/include --with-poco-libs=/usr/local/lib

sudo -Hu cool make -j$(nproc)

make install
mkdir -p /etc/coolwsd /usr/local/var/cache/coolwsd

chown cool:cool /usr/local/var/cache/coolwsd
admin_pwd=$(openssl rand -hex 6)

cat <<EOT > /lib/systemd/system/coolwsd.service
[Unit]
Description=Collabora Online WebSocket Daemon
After=network.target

[Service]
ExecStart=/opt/cool/online/coolwsd --o:sys_template_path=/opt/cool/online/systemplate \
--o:lo_template_path=/opt/lo/instdir --o:child_root_path=/opt/cool/online/jails \
--o:admin_console.username=admin --o:admin_console.password=$DOC_EDITOR_ADMIN_PWD \
--o:ssl.enable=false
User=cool

[Install]
WantedBy=multi-user.target
EOT

systemctl daemon-reload
systemctl enable coolwsd.service
systemctl start coolwsd.service
"

echo "Installation complete!"
echo "Admin password: $admin_pwd"
echo "Access at: https://localhost:9980"

