use super::*;

pub fn register_drive(components: &mut HashMap<String, ComponentConfig>) {
    components.insert(
        "drive".to_string(),
        ComponentConfig {
            name: "drive".to_string(),
            ports: vec![9100, 9101],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url: get_component_url("drive"),
            binary_name: Some("minio".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: vec![
                "cp {{DATA_PATH}}/mc {{BIN_PATH}}/mc && chmod +x {{BIN_PATH}}/mc".to_string(),
            ],
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: Vec::new(),
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: vec![
                "copy /Y {{DATA_PATH}}\\mc.exe {{BIN_PATH}}\\mc.exe".to_string(),
            ],
            env_vars: HashMap::from([
                ("MINIO_ROOT_USER".to_string(), "$DRIVE_ACCESSKEY".to_string()),
                ("MINIO_ROOT_PASSWORD".to_string(), "$DRIVE_SECRET".to_string()),
            ]),
            data_download_list: get_component_url("mc").map(|url| vec![url]).unwrap_or_default(),
            exec_cmd: "nohup {{BIN_PATH}}/minio server {{DATA_PATH}} --address 127.0.0.1:9100 --console-address 127.0.0.1:9101 --certs-dir {{CONF_PATH}}/drive/certs > {{LOGS_PATH}}/minio.log 2>&1 &".to_string(),
            check_cmd: "curl -sf --cacert {{CONF_PATH}}/drive/certs/CAs/ca.crt https://127.0.0.1:9100/minio/health/live >/dev/null 2>&1".to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\minio.exe server {{DATA_PATH}} --address 127.0.0.1:9100 --console-address 127.0.0.1:9101 --certs-dir {{CONF_PATH}}\\drive\\certs > {{LOGS_PATH}}\\minio.log 2>&1".to_string()),
            check_cmd_windows: Some("curl -sf --cacert {{CONF_PATH}}\\drive\\certs\\CAs\\ca.crt https://127.0.0.1:9100/minio/health/live >NUL 2>&1".to_string()),
            container: None,
        },
    );
}
pub fn register_tables(components: &mut HashMap<String, ComponentConfig>) {
    components.insert(
        "tables".to_string(),
        ComponentConfig {
            name: "tables".to_string(),
            ports: vec![5432],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url: get_component_url("tables"),
            binary_name: Some("postgres".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: vec![
                "chmod +x ./bin/*".to_string(),
                "id postgres 2>/dev/null || useradd -r -s /bin/false postgres 2>/dev/null; [ \"$(id -u)\" -eq 0 ] && chown -R postgres:postgres {{DATA_PATH}} {{LOGS_PATH}} 2>/dev/null || true".to_string(),
                // Root-aware wrapper: run as the `postgres` user when root, otherwise directly as the launching user (allows non-root dev stacks where no `postgres` user exists).
                "cat > {{BIN_PATH}}/pgwrap.sh <<'PGB_WRAP'\n#!/bin/bash\ncmd=\"$*\"\nif [ \"$(id -u)\" -eq 0 ]; then su -s /bin/bash postgres -c \"$cmd\"; else bash -c \"$cmd\"; fi\nPGB_WRAP\nchmod +x {{BIN_PATH}}/pgwrap.sh".to_string(),
                "if [ ! -d \"{{DATA_PATH}}/pgdata\" ]; then printf '%s\\n' '{{DB_PASSWORD}}' > {{LOGS_PATH}}/pwfile && {{BIN_PATH}}/pgwrap.sh \"cd {{BIN_PATH}} && ./bin/initdb -D {{DATA_PATH}}/pgdata -U gbuser --pwfile={{LOGS_PATH}}/pwfile\" && rm -f {{LOGS_PATH}}/pwfile; fi".to_string(),
                "echo \"data_directory = '{{DATA_PATH}}/pgdata'\" > {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"ident_file = '{{CONF_PATH}}/pg_ident.conf'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"port = 5432\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"listen_addresses = '*'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"ssl = on\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"ssl_cert_file = '{{CONF_PATH}}/system/certificates/tables/server.crt'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"ssl_key_file = '{{CONF_PATH}}/system/certificates/tables/server.key'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"ssl_ca_file = '{{CONF_PATH}}/system/certificates/ca/ca.crt'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"log_directory = '{{LOGS_PATH}}'\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"logging_collector = on\" >> {{CONF_PATH}}/postgresql.conf".to_string(),
                "echo \"hostssl all all all md5\" > {{CONF_PATH}}/pg_hba.conf".to_string(),
                "touch {{CONF_PATH}}/pg_ident.conf".to_string(),
                "{{BIN_PATH}}/pgwrap.sh \"cd {{BIN_PATH}} && ./bin/pg_ctl -D {{DATA_PATH}}/pgdata -l {{LOGS_PATH}}/postgres.log start -w -t 30\"".to_string(),
                "sleep 5".to_string(),
                "for i in $(seq 1 30); do {{BIN_PATH}}/pgwrap.sh \"cd {{BIN_PATH}} && ./bin/pg_isready -h localhost -p 5432 -d postgres\" >/dev/null 2>&1 && echo 'PostgreSQL is ready' && break || echo \"Waiting for PostgreSQL... attempt $i/30\" >&2; sleep 2; done".to_string(),
                "{{BIN_PATH}}/pgwrap.sh \"cd {{BIN_PATH}} && ./bin/pg_isready -h localhost -p 5432 -d postgres\" || { echo 'ERROR: PostgreSQL failed to start properly' >&2; cat {{LOGS_PATH}}/postgres.log >&2; exit 1; }".to_string(),
                "{{BIN_PATH}}/pgwrap.sh \"cd {{BIN_PATH}} && PGPASSWORD='{{DB_PASSWORD}}' ./bin/psql -h localhost -p 5432 -U gbuser -d postgres -c \\\"CREATE DATABASE botserver WITH OWNER gbuser\\\"\" 2>&1 | grep -v 'already exists' || true".to_string(),
            ],
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: vec![
                "chmod +x ./bin/*".to_string(),
                "if [ ! -d \"{{DATA_PATH}}/pgdata\" ]; then ./bin/initdb -A -D {{DATA_PATH}}/pgdata -U postgres; fi".to_string(),
            ],
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: vec![
                "if not exist \"{{DATA_PATH}}\\pgdata\\PG_VERSION\" \"{{BIN_PATH}}\\bin\\initdb.exe\" --encoding=UTF8 --no-locale -D \"{{DATA_PATH}}\\pgdata\" -U gbuser --auth=trust".to_string(),
                "echo data_directory = '{{DATA_PATH}}\\pgdata' > \"{{CONF_PATH}}\\postgresql.conf\"".to_string(),
                "echo port = 5432 >> \"{{CONF_PATH}}\\postgresql.conf\"".to_string(),
                "echo listen_addresses = '*' >> \"{{CONF_PATH}}\\postgresql.conf\"".to_string(),
                "echo log_directory = '{{LOGS_PATH}}' >> \"{{CONF_PATH}}\\postgresql.conf\"".to_string(),
                "echo logging_collector = on >> \"{{CONF_PATH}}\\postgresql.conf\"".to_string(),
                "echo host all all all trust > \"{{CONF_PATH}}\\pg_hba.conf\"".to_string(),
                "type nul > \"{{CONF_PATH}}\\pg_ident.conf\"".to_string(),
                "\"{{BIN_PATH}}\\bin\\pg_ctl.exe\" -D \"{{DATA_PATH}}\\pgdata\" -l \"{{LOGS_PATH}}\\postgresql.log\" start -w -t 30".to_string(),
                "\"{{BIN_PATH}}\\bin\\pg_isready.exe\" -h localhost -p 5432 -d postgres".to_string(),
                "set PGPASSWORD={{DB_PASSWORD}} && \"{{BIN_PATH}}\\bin\\psql.exe\" -h localhost -p 5432 -U gbuser -d postgres -c \"CREATE DATABASE botserver WITH OWNER gbuser\" 2>nul".to_string(),
            ],
            env_vars: HashMap::new(),
            data_download_list: Vec::new(),
            exec_cmd: "if [ $(id -u) -eq 0 ]; then id postgres 2>/dev/null || useradd -r -s /bin/false postgres; chown -R postgres:postgres {{DATA_PATH}} {{LOGS_PATH}} 2>/dev/null; su -s /bin/bash postgres -c \"cd {{BIN_PATH}} && ./bin/pg_ctl -D {{DATA_PATH}}/pgdata -l {{LOGS_PATH}}/postgres.log start -w -t 30\"; else cd {{BIN_PATH}} && ./bin/pg_ctl -D {{DATA_PATH}}/pgdata -l {{LOGS_PATH}}/postgres.log start -w -t 30; fi > {{LOGS_PATH}}/stdout.log 2>&1 &".to_string(),
            check_cmd: "if [ $(id -u) -eq 0 ]; then su -s /bin/bash postgres -c \"{{BIN_PATH}}/bin/pg_isready -h localhost -p 5432 -d postgres\"; else {{BIN_PATH}}/bin/pg_isready -h localhost -p 5432 -d postgres; fi >/dev/null 2>&1".to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\bin\\pg_ctl -D {{DATA_PATH}}\\pgdata -l {{LOGS_PATH}}\\postgresql.log start -w -t 30 > {{LOGS_PATH}}\\stdout.log 2>&1".to_string()),
            check_cmd_windows: Some("\"{{BIN_PATH}}\\bin\\pg_isready.exe\" -h localhost -p 5432 -d postgres >NUL 2>&1".to_string()),
            container: None,
        },
    );
}
pub fn register_cache(components: &mut HashMap<String, ComponentConfig>) {
    components.insert(
        "cache".to_string(),
        ComponentConfig {
            name: "cache".to_string(),
            ports: vec![6379],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url: get_component_url("cache"),
            binary_name: Some("valkey-server".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: vec![
                "mkdir -p {{BIN_PATH}}/bin && cd {{BIN_PATH}}/bin && tar -xzf {{CACHE_FILE}} --strip-components=1 -C {{BIN_PATH}}/bin 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-server 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-cli 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-benchmark 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-check-aof 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-check-rdb 2>/dev/null || true".to_string(),
                "chmod +x {{BIN_PATH}}/bin/valkey-sentinel 2>/dev/null || true".to_string(),
            ],
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: Vec::new(),
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: Vec::new(),
            env_vars: HashMap::new(),
            data_download_list: Vec::new(),
            exec_cmd: "nohup {{BIN_PATH}}/bin/valkey-server --port 6379 --bind 127.0.0.1 --dir {{DATA_PATH}} --logfile {{LOGS_PATH}}/valkey.log --daemonize yes > {{LOGS_PATH}}/valkey-startup.log 2>&1".to_string(),
            check_cmd: "pgrep -x valkey-server >/dev/null 2>&1".to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\redis-server.exe --port 6379 --bind 127.0.0.1 --dir {{DATA_PATH}} --logfile {{LOGS_PATH}}\\valkey.log > {{LOGS_PATH}}\\valkey-startup.log 2>&1".to_string()),
            check_cmd_windows: Some("tasklist /FI \"IMAGENAME eq redis-server.exe\" 2>NUL | findstr /I \"redis\" >NUL".to_string()),
            container: None,
        },
    );
}
pub fn register_llm(components: &mut HashMap<String, ComponentConfig>) {
    let download_url = get_llama_cpp_url();

    if download_url.is_none() {
        warn!("No llama.cpp binary available for this platform");
        warn!("Local LLM will not be available - use external API instead");
    }

    info!(
        "LLM component using llama.cpp {} for this platform",
        LLAMA_CPP_VERSION
    );

    components.insert(
        "llm".to_string(),
        ComponentConfig {
            name: "llm".to_string(),
            ports: vec![8081, 8082],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url,
            binary_name: Some("llama-server".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: Vec::new(),
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: Vec::new(),
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: Vec::new(),
            env_vars: HashMap::new(),
            data_download_list: vec![
                get_model_url("deepseek_small").unwrap_or_default(),
                get_model_url("bge_embedding").unwrap_or_default(),
            ],
            exec_cmd: "nohup {{BIN_PATH}}/build/bin/llama-server --port 8081 --ssl-key-file {{CONF_PATH}}/system/certificates/llm/server.key --ssl-cert-file {{CONF_PATH}}/system/certificates/llm/server.crt -m {{DATA_PATH}}/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf --ubatch-size 512 > {{LOGS_PATH}}/llm.log 2>&1 & nohup {{BIN_PATH}}/build/bin/llama-server --port 8082 --ssl-key-file {{CONF_PATH}}/system/certificates/embedding/server.key --ssl-cert-file {{CONF_PATH}}/system/certificates/embedding/server.crt -m {{DATA_PATH}}/bge-small-en-v1.5-f32.gguf --embeddings --pooling mean --n-gpu-layers 0 --ctx-size 512 --ubatch-size 512 > {{LOGS_PATH}}/embedding.log 2>&1 &".to_string(),
            check_cmd: "curl -f -k --connect-timeout 2 -m 5 https://localhost:8081/health >/dev/null 2>&1 && curl -f -k --connect-timeout 2 -m 5 https://localhost:8082/health >/dev/null 2>&1".to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\build\\bin\\llama-server --port 8081 --ssl-key-file {{CONF_PATH}}\\system\\certificates\\llm\\server.key --ssl-cert-file {{CONF_PATH}}\\system\\certificates\\llm\\server.crt -m {{DATA_PATH}}\\DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf --ubatch-size 512 > {{LOGS_PATH}}\\llm.log 2>&1".to_string()),
            check_cmd_windows: Some("curl -f -k --connect-timeout 2 -m 5 https://localhost:8081/health >NUL 2>&1".to_string()),
            container: None,
        },
    );
}
pub fn register_email(components: &mut HashMap<String, ComponentConfig>) {
    components.insert(
        "email".to_string(),
        ComponentConfig {
            name: "email".to_string(),
            ports: vec![25, 143, 465, 993, 8025],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url: get_component_url("email"),
            binary_name: Some("stalwart-mail".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: Vec::new(),
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: Vec::new(),
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: Vec::new(),
            env_vars: HashMap::from([
                ("STALWART_TLS_ENABLE".to_string(), "true".to_string()),
                ("STALWART_TLS_CERT".to_string(), "{{CONF_PATH}}/system/certificates/email/server.crt".to_string()),
                ("STALWART_TLS_KEY".to_string(), "{{CONF_PATH}}/system/certificates/email/server.key".to_string()),
            ]),
            data_download_list: Vec::new(),
            exec_cmd: "{{BIN_PATH}}/stalwart-mail --config {{CONF_PATH}}/email/config.toml".to_string(),
            check_cmd: "curl -f -k --connect-timeout 2 -m 5 https://localhost:8025/health >/dev/null 2>&1".to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\stalwart-mail.exe --config {{CONF_PATH}}\\email\\config.toml".to_string()),
            check_cmd_windows: Some("curl -f -k --connect-timeout 2 -m 5 https://localhost:8025/health >NUL 2>&1".to_string()),
            container: None,
        },
    );
}
pub fn register_proxy(components: &mut HashMap<String, ComponentConfig>) {
    components.insert(
        "proxy".to_string(),
        ComponentConfig {
            name: "proxy".to_string(),
            ports: vec![80, 443],
            dependencies: Vec::new(),
            linux_packages: Vec::new(),
            macos_packages: Vec::new(),
            windows_packages: Vec::new(),
            download_url: get_component_url("proxy"),
            binary_name: Some("caddy".to_string()),
            pre_install_cmds_linux: Vec::new(),
            post_install_cmds_linux: vec![
                "setcap 'cap_net_bind_service=+ep' {{BIN_PATH}}/caddy".to_string(),
            ],
            pre_install_cmds_macos: Vec::new(),
            post_install_cmds_macos: Vec::new(),
            pre_install_cmds_windows: Vec::new(),
            post_install_cmds_windows: Vec::new(),
            env_vars: HashMap::from([(
                "XDG_DATA_HOME".to_string(),
                "{{DATA_PATH}}".to_string(),
            )]),
            data_download_list: Vec::new(),
            exec_cmd: "{{BIN_PATH}}/caddy run --config {{CONF_PATH}}/Caddyfile".to_string(),
            check_cmd: "curl -f --connect-timeout 2 -m 5 http://localhost >/dev/null 2>&1"
                .to_string(),
            exec_cmd_windows: Some("start /B {{BIN_PATH}}\\caddy.exe run --config {{CONF_PATH}}\\Caddyfile".to_string()),
            check_cmd_windows: Some("curl -f --connect-timeout 2 -m 5 http://localhost >NUL 2>&1".to_string()),
            container: None,
        },
    );
}
