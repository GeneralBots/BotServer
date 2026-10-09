use super::*;

impl ManifestSection {
    pub fn new(name: &str, section_type: SectionType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            section_type,
            status: SectionStatus::Pending,
            current_step: 0,
            total_steps: 0,
            global_step_start: 0,
            duration_seconds: None,
            started_at: None,
            completed_at: None,
            items: Vec::new(),
            item_groups: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn with_steps(mut self, total: u32) -> Self {
        self.total_steps = total;
        self
    }

    pub fn add_item(&mut self, item: ManifestItem) {
        self.total_steps += 1;
        self.items.push(item);
    }

    pub fn add_item_group(&mut self, group: ItemGroup) {
        self.total_steps += 1;
        self.item_groups.push(group);
    }

    pub fn add_child(&mut self, child: ManifestSection) {
        self.total_steps += child.total_steps;
        self.children.push(child);
    }

    pub fn start(&mut self) {
        self.status = SectionStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = SectionStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.current_step = self.total_steps;
        if let Some(started) = self.started_at {
            self.duration_seconds = Some((Utc::now() - started).num_seconds() as u64);
        }
    }

    pub fn increment_step(&mut self) {
        self.current_step += 1;
    }
}
impl ManifestItem {
    pub fn new(name: &str, item_type: ItemType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            item_type,
            status: ItemStatus::Pending,
            details: None,
            duration_seconds: None,
            started_at: None,
            completed_at: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn start(&mut self) {
        self.status = ItemStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = ItemStatus::Completed;
        self.completed_at = Some(Utc::now());
        if let Some(started) = self.started_at {
            self.duration_seconds = Some((Utc::now() - started).num_seconds() as u64);
        }
    }
}
pub(super) fn section_to_web_json(section: &ManifestSection) -> serde_json::Value {
    let checkbox = match section.status {
        SectionStatus::Completed => "[x]",
        SectionStatus::Running => "[>]",
        _ => "[ ]",
    };
    let global_current = section.global_step_start + section.current_step;
    serde_json::json!({
        "id": section.id,
        "name": section.name,
        "checkbox": checkbox,
        "type": format!("{:?}", section.section_type),
        "status": format!("{:?}", section.status),
        "progress": {
            "current": section.current_step,
            "total": section.total_steps,
            "display": format!("Step {}/{}", section.current_step, section.total_steps),
            "global_current": global_current,
            "global_start": section.global_step_start
        },
        "duration": section.duration_seconds.map(format_duration),
        "duration_seconds": section.duration_seconds,
        "items": section.items.iter().map(|i| {
            let ic = match i.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
            serde_json::json!({
                "id": i.id, "name": i.name, "checkbox": ic,
                "type": format!("{:?}", i.item_type), "status": format!("{:?}", i.status),
                "details": i.details, "duration": i.duration_seconds.map(format_duration),
                "duration_seconds": i.duration_seconds
            })
        }).collect::<Vec<_>>(),
        "item_groups": section.item_groups.iter().map(|g| {
            let gc = match g.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
            serde_json::json!({
                "id": g.id, "name": g.display_name(), "items": g.items, "checkbox": gc,
                "status": format!("{:?}", g.status),
                "duration": g.duration_seconds.map(format_duration), "duration_seconds": g.duration_seconds
            })
        }).collect::<Vec<_>>(),
        "children": section.children.iter().map(section_to_web_json).collect::<Vec<_>>()
    })
}
pub(super) fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{} sec", seconds)
    } else if seconds < 3600 {
        format!("{} min", seconds / 60)
    } else {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        format!("{} hr {} min", hours, mins)
    }
}
