#!/usr/bin/env bash

# Define container limits in an associative array
declare -A container_limits=(
    # Pattern       Memory    CPU Allowance
     ["*tables*"]="4096MB:100ms/100ms"
     ["*dns*"]="2048MB:100ms/100ms"
     ["*doc-editor*"]="512MB:10ms/100ms"
     ["*proxy*"]="2048MB:100ms/100ms"
     ["*directory*"]="1024MB:50ms/100ms"
     ["*drive*"]="4096MB:50ms/100ms"
     ["*email*"]="4096MB:100ms/100ms"
     ["*webmail*"]="4096MB:100ms/100ms"
     ["*bot*"]="4096MB:50ms/100ms"
     ["*meeting*"]="4096MB:100ms/100ms"
     ["*alm*"]="512MB:50ms/100ms"
     ["*alm-ci*"]="4096MB:100ms/100ms"
     ["*system*"]="4096MB:50ms/100ms"
     ["*mailer*"]="4096MB:25ms/100ms"
)

# Default values (for containers that don't match any pattern)
DEFAULT_MEMORY="1024MB"
DEFAULT_CPU_ALLOWANCE="15ms/100ms"
CPU_COUNT=2
CPU_PRIORITY=10

for pattern in "${!container_limits[@]}"; do
    echo "Configuring $container..."

    memory=$DEFAULT_MEMORY
    cpu_allowance=$DEFAULT_CPU_ALLOWANCE

    # Configure all containers
    for container in $(lxc list -c n --format csv); do
    # Check if container matches any pattern
        if [[ $container == $pattern ]]; then
            IFS=':' read -r memory cpu_allowance <<< "${container_limits[$pattern]}"

            # Apply configuration
            lxc config set "$container" limits.memory "$memory"
            lxc config set "$container" limits.cpu.allowance "$cpu_allowance"
            lxc config set "$container" limits.cpu "$CPU_COUNT"
            lxc config set "$container" limits.cpu.priority "$CPU_PRIORITY"

            echo "Restarting $container..."
            lxc restart "$container"

            lxc config show "$container" | grep -E "memory|cpu"
            break
        fi
    done
done
