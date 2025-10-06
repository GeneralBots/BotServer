lxc list --format json | jq -r '.[].name' | while read container; do
    echo -n "$container: "
    lxc exec $container -- df -h / --output=used < /dev/null | tail -n1
done

du -h --max-depth=1 "." 2>/dev/null | sort -rh | head -n 50 | awk '{printf "%-10s %s\n", $1, $2}'
