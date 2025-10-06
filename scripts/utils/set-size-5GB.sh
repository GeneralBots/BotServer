lxc config device override $CONTAINER_NAME root
lxc config device set  $CONTAINER_NAME root size 6GB

zpool set autoexpand=on default
zpool online -e default /var/snap/lxd/common/lxd/disks/default.img
zpool list
zfs list
