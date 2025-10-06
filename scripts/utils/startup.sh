#!/bin/bash

# Disable shell timeout

sed -i '/TMOUT/d' /etc/profile /etc/bash.bashrc /etc/profile.d/*
echo 'export TMOUT=0' > /etc/profile.d/notimeout.sh
chmod +x /etc/profile.d/notimeout.sh
sed -i '/pam_exec.so/s/quiet/quiet set_timeout=0/' /etc/pam.d/sshd 2>/dev/null
source /etc/profile

