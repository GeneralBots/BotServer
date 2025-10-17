do not comment or echo anything

keep lines condensed
always call it <kind> not own name. Eg.: proxy instead of Caddy. alm instead of forgejo.
use KISS priciple

use local /opt/gbo/{logs, data, conf} exposed as
    HOST_BASE="/opt/gbo/tenants/$PARAM_TENANT/<kind>"
    HOST_DATA="$HOST_BASE/data"
    HOST_CONF="$HOST_BASE/conf"
    HOST_LOGS="$HOST_BASE/logs"
    instead of using app original paths.
and use /opt/gbo/bin to put local binaries of installations
during sh exection, never touch files in /opt/gbo/{logs, data, conf}
use wget
use gbuser as system user
