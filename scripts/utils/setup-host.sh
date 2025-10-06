
# Host
sudo lxc config set core.trust_password "$LXC_TRUST_PASSWORD"

# ALM-CI
lxc remote add bot 10.16.164.? --accept-certificate --password "$LXC_TRUST_PASSWORD"
