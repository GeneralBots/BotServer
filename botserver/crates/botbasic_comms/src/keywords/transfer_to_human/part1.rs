use super::*;

pub fn is_crm_enabled(bot_id: Uuid, work_path: &str) -> bool {
    let config_path = PathBuf::from(work_path)
        .join(format!("{}.gbai", bot_id))
        .join("config.csv");

    if !config_path.exists() {
        let alt_path = PathBuf::from(work_path).join("config.csv");
        if alt_path.exists() {
            return check_config_for_crm(&alt_path);
        }
        warn!("Config file not found: {}", config_path.display());
        return false;
    }

    check_config_for_crm(&config_path)
}
pub(super) fn check_config_for_crm(config_path: &PathBuf) -> bool {
    match std::fs::read_to_string(config_path) {
        Ok(content) => {
            for line in content.lines() {
                let line_lower = line.to_lowercase();

                if (line_lower.contains("crm-enabled") || line_lower.contains("crm_enabled"))
                    && line_lower.contains("true")
                {
                    return true;
                }

                if line_lower.contains("transfer") && line_lower.contains("true") {
                    return true;
                }
            }
            false
        }
        Err(e) => {
            log::error!("Failed to read config file: {}", e);
            false
        }
    }
}
pub fn read_attendants(bot_id: Uuid, work_path: &str) -> Vec<Attendant> {
    let attendant_path = PathBuf::from(work_path)
        .join(format!("{}.gbai", bot_id))
        .join("attendant.csv");

    if !attendant_path.exists() {
        let alt_path = PathBuf::from(work_path).join("attendant.csv");
        if alt_path.exists() {
            return parse_attendants_csv(&alt_path);
        }
        warn!("Attendants file not found: {}", attendant_path.display());
        return Vec::new();
    }

    parse_attendants_csv(&attendant_path)
}
pub(super) fn parse_attendants_csv(path: &PathBuf) -> Vec<Attendant> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let mut attendants = Vec::new();
            let mut lines = content.lines();

            let header = lines.next().unwrap_or("");
            let headers: Vec<String> = header.split(',').map(|s| s.trim().to_lowercase()).collect();

            let id_idx = headers.iter().position(|h| h == "id").unwrap_or(0);
            let name_idx = headers.iter().position(|h| h == "name").unwrap_or(1);
            let channel_idx = headers.iter().position(|h| h == "channel").unwrap_or(2);
            let pref_idx = headers.iter().position(|h| h == "preferences").unwrap_or(3);
            let dept_idx = headers.iter().position(|h| h == "department");
            let alias_idx = headers.iter().position(|h| h == "aliases" || h == "alias");

            for line in lines {
                if line.trim().is_empty() {
                    continue;
                }

                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 4 {
                    let department = dept_idx.and_then(|i| parts.get(i).map(|s| (*s).to_string()));
                    let aliases = alias_idx
                        .and_then(|i| parts.get(i))
                        .map(|s| s.split(';').map(|a| a.trim().to_lowercase()).collect())
                        .unwrap_or_default();

                    attendants.push(Attendant {
                        id: (*parts.get(id_idx).unwrap_or(&"")).to_string(),
                        name: (*parts.get(name_idx).unwrap_or(&"")).to_string(),
                        channel: (*parts.get(channel_idx).unwrap_or(&"all")).to_string(),
                        preferences: (*parts.get(pref_idx).unwrap_or(&"")).to_string(),
                        department,
                        aliases,
                        status: AttendantStatus::Online,
                    });
                }
            }

            info!("Loaded {} attendants from CSV", attendants.len());
            attendants
        }
        Err(e) => {
            log::error!("Failed to read attendant file: {}", e);
            Vec::new()
        }
    }
}
pub fn find_attendant<'a>(
    attendants: &'a [Attendant],
    name: Option<&str>,
    department: Option<&str>,
) -> Option<&'a Attendant> {
    if let Some(search_name) = name {
        let search_lower = search_name.to_lowercase();

        if let Some(att) = attendants
            .iter()
            .find(|a| a.name.to_lowercase() == search_lower)
        {
            return Some(att);
        }

        if let Some(att) = attendants
            .iter()
            .find(|a| a.name.to_lowercase().contains(&search_lower))
        {
            return Some(att);
        }

        if let Some(att) = attendants
            .iter()
            .find(|a| a.aliases.contains(&search_lower))
        {
            return Some(att);
        }

        if let Some(att) = attendants
            .iter()
            .find(|a| a.id.to_lowercase() == search_lower)
        {
            return Some(att);
        }
    }

    if let Some(dept) = department {
        let dept_lower = dept.to_lowercase();

        if let Some(att) = attendants.iter().find(|a| {
            a.department
                .as_ref()
                .map(|d| d.to_lowercase() == dept_lower)
                .unwrap_or(false)
                && a.status == AttendantStatus::Online
        }) {
            return Some(att);
        }

        if let Some(att) = attendants.iter().find(|a| {
            a.preferences.to_lowercase().contains(&dept_lower)
                && a.status == AttendantStatus::Online
        }) {
            return Some(att);
        }
    }

    attendants
        .iter()
        .find(|a| a.status == AttendantStatus::Online)
}
pub(super) fn priority_to_int(priority: Option<&str>) -> i32 {
    match priority.map(|p| p.to_lowercase()).as_deref() {
        Some("urgent") => 3,
        Some("high") => 2,
        Some("low") => 0,
        _ => 1,
    }
}
