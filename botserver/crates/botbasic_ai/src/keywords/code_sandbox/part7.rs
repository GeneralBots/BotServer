

pub fn generate_python_lxc_config() -> String {
    r"
# LXC configuration for Python sandbox
lxc.include = /usr/share/lxc/config/common.conf
lxc.arch = linux64

# Container name template
lxc.uts.name = gb-sandbox-python

# Root filesystem
lxc.rootfs.path = dir:/var/lib/lxc/gb-sandbox-python/rootfs

# Network - isolated by default
lxc.net.0.type = empty

# Resource limits
lxc.cgroup2.memory.max = 256M
lxc.cgroup2.cpu.max = 50000 100000

# Security
lxc.cap.drop = sys_admin sys_boot sys_module sys_time
lxc.apparmor.profile = generated
lxc.seccomp.profile = /usr/share/lxc/config/common.seccomp

# Mount points - minimal
lxc.mount.auto = proc:mixed sys:ro
lxc.mount.entry = /usr/bin/python3 usr/bin/python3 none ro,bind 0 0
lxc.mount.entry = /usr/lib/python3 usr/lib/python3 none ro,bind 0 0
lxc.mount.entry = tmpfs tmp tmpfs defaults 0 0
"
    .to_string()
}
pub fn generate_node_lxc_config() -> String {
    r"
# LXC configuration for Node.js sandbox
lxc.include = /usr/share/lxc/config/common.conf
lxc.arch = linux64

# Container name template
lxc.uts.name = gb-sandbox-node

# Root filesystem
lxc.rootfs.path = dir:/var/lib/lxc/gb-sandbox-node/rootfs

# Network - isolated by default
lxc.net.0.type = empty

# Resource limits
lxc.cgroup2.memory.max = 256M
lxc.cgroup2.cpu.max = 50000 100000

# Security
lxc.cap.drop = sys_admin sys_boot sys_module sys_time
lxc.apparmor.profile = generated
lxc.seccomp.profile = /usr/share/lxc/config/common.seccomp

# Mount points - minimal
lxc.mount.auto = proc:mixed sys:ro
lxc.mount.entry = /usr/bin/node usr/bin/node none ro,bind 0 0
lxc.mount.entry = /usr/lib/node_modules usr/lib/node_modules none ro,bind 0 0
lxc.mount.entry = tmpfs tmp tmpfs defaults 0 0
"
    .to_string()
}
