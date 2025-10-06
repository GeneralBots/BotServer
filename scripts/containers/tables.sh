
HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/tables"
HOST_DATA="$HOST_BASE/data"
HOST_CONF="$HOST_BASE/conf"
HOST_LOGS="$HOST_BASE/logs"

mkdir -p "$HOST_DATA" "$HOST_CONF" "$HOST_LOGS"

lxc launch images:debian/12 "$PARAM_TENANT"-tables -c security.privileged=true

until lxc exec "$PARAM_TENANT"-tables -- test -f /bin/bash; do
    sleep 5
done
sleep 10

lxc exec "$PARAM_TENANT"-tables -- bash -c "
set -e
export DEBIAN_FRONTEND=noninteractive

apt-get update
apt-get install -y wget gnupg2 sudo lsb-release curl

sudo apt install -y postgresql-common
sudo /usr/share/postgresql-common/pgdg/apt.postgresql.org.sh
apt install -y postgresql

# TODO: Open listener on *.

until sudo -u postgres psql -p $PARAM_TABLES_PORT -c '\q' 2>/dev/null; do
    echo \"Waiting for PostgreSQL to start on port $PARAM_TABLES_PORT...\"
    sleep 3
done

sudo -u postgres psql -p $PARAM_TABLES_PORT -c \"CREATE USER $PARAM_TENANT WITH PASSWORD '$PARAM_TABLES_PASSWORD';\"
sudo -u postgres psql -p $PARAM_TABLES_PORT -c \"CREATE DATABASE ${PARAM_TENANT}_db OWNER $PARAM_TENANT;\"
sudo -u postgres psql -p $PARAM_TABLES_PORT -c \"GRANT ALL PRIVILEGES ON DATABASE ${PARAM_TENANT}_db TO $PARAM_TENANT;\"

"


lxc config device remove "$PARAM_TENANT"-tables postgres-proxy 2>/dev/null || true
lxc config device add "$PARAM_TENANT"-tables postgres-proxy proxy \
    listen=tcp:0.0.0.0:"$PARAM_TABLES_PORT" \
    connect=tcp:127.0.0.1:"$PARAM_TABLES_PORT"

echo "PostgreSQL setup completed successfully!"
echo "Database: ${PARAM_TENANT}_db"
echo "User: $PARAM_TENANT"
echo "Password: $PARAM_TABLES_PASSWORD"
echo "Port: $PARAM_TABLES_PORT"
