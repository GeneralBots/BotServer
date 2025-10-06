df -h
printf "%-20s %-10s %-10s %-10s %-6s %s\n" "CONTAINER" "USED" "AVAIL" "TOTAL" "USE%" "MOUNT"
for container in $(lxc list -c n --format csv); do
    disk_info=$(lxc exec $container -- df -h / --output=used,avail,size,pcent | tail -n 1)
    printf "%-20s %s\n" "$container" "$disk_info"
done

#!/bin/bash

# Directory to analyze
TARGET_DIR="/opt/gbo/tenants/pragmatismo"

echo "Calculating sizes for directories in $TARGET_DIR..."
echo ""

# Check if directory exists
if [ ! -d "$TARGET_DIR" ]; then
  echo "Error: Directory $TARGET_DIR does not exist"
  exit 1
fi

# Get the size of each subdirectory
echo "Directory Size Report:"
echo "----------------------"
du -h --max-depth=1 "$TARGET_DIR" | sort -hr | awk -F'\t' '{printf "%-50s %s\n", $2, $1}'

echo ""
echo "Total size:"
du -sh "$TARGET_DIR"