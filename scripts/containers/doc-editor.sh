#!/bin/bash

HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/doc-editor"

lxc launch images:debian/12 "${PARAM_TENANT}-doc-editor" \
    -c security.privileged=true \
    -c limits.cpu=2 \
    -c limits.memory=4096MB \
    
sleep 10

lxc exec "$PARAM_TENANT"-doc-editor -- bash -c "

cd /usr/share/keyrings
wget https://collaboraoffice.com/downloads/gpg/collaboraonline-release-keyring.gpg

cat << EOF > /etc/apt/sources.list.d/collaboraonline.sources
Types: deb
URIs: https://www.collaboraoffice.com/repos/CollaboraOnline/24.04/customer-deb-$customer_hash
Suites: ./
Signed-By: /usr/share/keyrings/collaboraonline-release-keyring.gpg
EOF

apt update && apt install coolwsd collabora-online-brand
"

lxc config device remove "$PARAM_TENANT"-doc-editor doc-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-doc-editor doc-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_DOC_PORT" \
    connect=tcp:127.0.0.1:9980