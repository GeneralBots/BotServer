#!/bin/bash

# Cleanup script for Ubuntu Server and LXC containers
# Run with sudo privileges

echo "Starting system cleanup..."

### Host System Cleanup ###
echo -e "\n[ HOST SYSTEM CLEANUP ]"

# Package manager cache
echo "Cleaning package cache..."
apt clean
apt autoclean
apt autoremove -y

# Journal logs
echo "Cleaning journal logs..."
journalctl --vacuum-time=2d 2>/dev/null

# Temporary files
echo "Cleaning temporary files..."
rm -rf /tmp/* /var/tmp/*

# Thumbnail cache
echo "Cleaning thumbnail cache..."
rm -rf ~/.cache/thumbnails/* /root/.cache/thumbnails/*

# DNS cache
echo "Flushing DNS cache..."
systemd-resolve --flush-caches 2>/dev/null || true

# Old kernels (keep 2 latest)
echo "Removing old kernels..."
apt purge -y $(dpkg -l | awk '/^ii linux-image-*/{print $2}' | grep -v $(uname -r) | head -n -2) 2>/dev/null

# Crash reports
echo "Clearing crash reports..."
rm -f /var/crash/*

### LXC Containers Cleanup ###
echo -e "\n[ LXC CONTAINERS CLEANUP ]"

# Check if LXC is installed
if command -v lxc >/dev/null 2>&1; then
    for container in $(lxc list -c n --format csv | grep -v "^$"); do
        echo -e "\nCleaning container: $container"
        
        # Execute cleanup commands in container
        lxc exec "$container" -- bash -c "
            echo 'Cleaning package cache...'
            apt clean && apt autoclean && apt autoremove -y
            
            echo 'Cleaning temporary files...'
            rm -rf /tmp/* /var/tmp/*
            
            echo 'Cleaning logs...'
            rm -rf /opt/gbo/logs/*
            
            echo 'Cleaning journal logs...'
            journalctl --vacuum-time=1d 2>/dev/null || true
            
            echo 'Cleaning thumbnail cache...'
            rm -rf /home/*/.cache/thumbnails/* /root/.cache/thumbnails/*
        " 2>/dev/null
    done
else
    echo "LXC not installed, skipping container cleanup."
fi

echo -e "\nCleanup completed!"