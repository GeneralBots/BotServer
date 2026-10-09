use super::*;

impl TaskManifest {
    pub fn new(app_name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            app_name: app_name.to_string(),
            description: description.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: ManifestStatus::Planning,
            current_status: CurrentStatus {
                title: description.to_string(),
                current_action: None,
                decision_point: None,
            },
            sections: Vec::new(),
            total_steps: 0,
            completed_steps: 0,
            runtime_seconds: 0,
            estimated_seconds: 0,
            terminal_output: Vec::new(),
            processing_stats: ProcessingStats::default(),
        }
    }

    pub fn set_current_action(&mut self, action: &str) {
        self.current_status.current_action = Some(action.to_string());
        self.updated_at = Utc::now();
    }

    pub fn set_decision_point(&mut self, current: u32, total: u32, message: &str) {
        self.current_status.decision_point = Some(DecisionPoint {
            step_current: current,
            step_total: total,
            message: message.to_string(),
        });
        self.updated_at = Utc::now();
    }

    pub fn add_section(&mut self, mut section: ManifestSection) {
        section.global_step_start = self.total_steps;
        let mut child_offset = self.total_steps;
        for child in &mut section.children {
            child.global_step_start = child_offset;
            child_offset += child.total_steps;
        }
        self.total_steps += section.total_steps;
        for child in &section.children {
            self.total_steps += child.total_steps;
        }
        self.sections.push(section);
        self.updated_at = Utc::now();
    }

    pub fn start(&mut self) {
        self.status = ManifestStatus::Running;
        self.updated_at = Utc::now();
    }

    pub fn complete(&mut self) {
        self.status = ManifestStatus::Completed;
        self.completed_steps = self.total_steps;
        self.updated_at = Utc::now();
    }

    pub fn recalculate_global_steps(&mut self) {
        let mut offset = 0u32;
        for section in &mut self.sections {
            section.global_step_start = offset;
            let mut child_offset = offset;
            for child in &mut section.children {
                child.global_step_start = child_offset;
                child_offset += child.total_steps;
            }
            offset += section.total_steps;
            for child in &section.children {
                offset += child.total_steps;
            }
        }
        self.total_steps = offset;
        self.updated_at = Utc::now();
    }

    pub fn fail(&mut self) {
        self.status = ManifestStatus::Failed;
        self.updated_at = Utc::now();
    }

    pub fn add_terminal_line(&mut self, content: &str, line_type: TerminalLineType) {
        self.terminal_output.push(TerminalLine {
            timestamp: Utc::now(),
            content: content.to_string(),
            line_type,
        });
        self.updated_at = Utc::now();
    }

    pub fn update_section_status(&mut self, section_id: &str, status: SectionStatus) {
        for section in &mut self.sections {
            if section.id == section_id {
                section.status = status.clone();
                if status == SectionStatus::Completed {
                    section.completed_at = Some(Utc::now());
                    self.completed_steps += section.total_steps;
                }
                break;
            }
            for child in &mut section.children {
                if child.id == section_id {
                    child.status = status.clone();
                    if status == SectionStatus::Completed {
                        child.completed_at = Some(Utc::now());
                        self.completed_steps += child.total_steps;
                    }
                    break;
                }
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn update_item_status(&mut self, section_id: &str, item_id: &str, status: ItemStatus) {
        for section in &mut self.sections {
            if section.id == section_id {
                for item in &mut section.items {
                    if item.id == item_id {
                        item.status = status;
                        if status == ItemStatus::Completed {
                            item.completed_at = Some(Utc::now());
                        }
                        return;
                    }
                }
            }
            for child in &mut section.children {
                if child.id == section_id {
                    for item in &mut child.items {
                        if item.id == item_id {
                            item.status = status;
                            if status == ItemStatus::Completed {
                                item.completed_at = Some(Utc::now());
                            }
                            return;
                        }
                    }
                }
            }
        }
        self.updated_at = Utc::now();
    }

    pub fn update_processing_stats(&mut self, stats: ProcessingStats) {
        self.processing_stats = stats;
        self.updated_at = Utc::now();
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.total_steps == 0 {
            return 0.0;
        }
        (self.completed_steps as f64 / self.total_steps as f64) * 100.0
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# TASK.md - {}\n\n", self.app_name));
        md.push_str(&format!("**Description:** {}\n\n", self.description));
        md.push_str(&format!("**Status:** {:?}\n", self.status));
        md.push_str(&format!(
            "**Progress:** {}/{} steps ({}%)\n\n",
            self.completed_steps,
            self.total_steps,
            self.progress_percentage() as u32
        ));
        md.push_str("## Artifacts\n\n");
        for section in &self.sections {
            md.push_str(&format!("### {} - {:?}\n", section.name, section.status));
            md.push_str(&format!("- Steps: {}/{}\n", section.current_step, section.total_steps));
            if !section.items.is_empty() {
                md.push_str("- Items:\n");
                for item in &section.items {
                    md.push_str(&format!("  - {} ({:?}): {:?}\n", item.name, item.item_type, item.status));
                }
            }
            for child in &section.children {
                md.push_str(&format!("  #### {} - {:?}\n", child.name, child.status));
                md.push_str(&format!("  - Steps: {}/{}\n", child.current_step, child.total_steps));
                if !child.items.is_empty() {
                    md.push_str("  - Items:\n");
                    for item in &child.items {
                        md.push_str(&format!("   - {} ({:?}): {:?}\n", item.name, item.item_type, item.status));
                    }
                }
            }
            md.push('\n');
        }
        md
    }

    pub fn to_web_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "app_name": self.app_name,
            "description": self.description,
            "status": {
                "title": self.current_status.title,
                "runtime_display": format_duration(self.runtime_seconds),
                "estimated_display": format_duration(self.estimated_seconds),
                "current_action": self.current_status.current_action,
                "decision_point": self.current_status.decision_point.as_ref().map(|dp| serde_json::json!({
                    "step_current": dp.step_current,
                    "step_total": dp.step_total,
                    "message": dp.message
                }))
            },
            "progress": {
                "current": self.completed_steps,
                "total": self.total_steps,
                "percentage": self.progress_percentage()
            },
            "sections": self.sections.iter().map(section_to_web_json).collect::<Vec<_>>(),
            "terminal": {
                "lines": self.terminal_output.iter().map(|l| serde_json::json!({
                    "content": l.content,
                    "type": format!("{:?}", l.line_type).to_lowercase(),
                    "timestamp": l.timestamp.to_rfc3339()
                })).collect::<Vec<_>>(),
                "stats": {
                    "processed": self.processing_stats.data_points_processed,
                    "speed": format!("{:.1} sources/min", self.processing_stats.sources_per_min),
                    "estimated_completion": format_duration(self.processing_stats.estimated_remaining_seconds)
                }
            }
        })
    }

    pub fn to_task_md(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# TASK.md - {}\n\n", self.app_name));
        md.push_str("## STATUS\n");
        md.push_str(&format!("- {}\n", self.current_status.title));
        if let Some(ref action) = self.current_status.current_action {
            md.push_str(&format!("  - [>] {}\n", action));
        }
        if let Some(ref dp) = self.current_status.decision_point {
            md.push_str(&format!("  - [ ] Decision Point (Step {}/{}) - {}\n", dp.step_current, dp.step_total, dp.message));
        }
        md.push_str("\n## PROGRESS LOG\n");
        for section in &self.sections {
            let checkbox = match section.status {
                SectionStatus::Completed => "[x]",
                SectionStatus::Running => "[>]",
                _ => "[ ]",
            };
            let global_step = section.global_step_start + section.current_step;
            md.push_str(&format!("- {} {} (Step {}/{})\n", checkbox, section.name, global_step, self.total_steps));
            for child in &section.children {
                let child_checkbox = match child.status {
                    SectionStatus::Completed => "[x]",
                    SectionStatus::Running => "[>]",
                    _ => "[ ]",
                };
                md.push_str(&format!("  - {} {} (Step {}/{})\n", child_checkbox, child.name, child.current_step, child.total_steps));
                for group in &child.item_groups {
                    let gc = match group.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
                    let dur = group.duration_seconds.map(|s| format!(" - Duration: {} min", s / 60)).unwrap_or_default();
                    md.push_str(&format!("   - {} {}{}\n", gc, group.display_name(), dur));
                }
                for item in &child.items {
                    let ic = match item.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
                    let dur = item.duration_seconds.map(|s| format!(" - Duration: {}s", s)).unwrap_or_default();
                    md.push_str(&format!("   - {} {}{}\n", ic, item.name, dur));
                }
            }
            for group in &section.item_groups {
                let gc = match group.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
                let dur = group.duration_seconds.map(|s| format!(" - Duration: {} min", s / 60)).unwrap_or_default();
                md.push_str(&format!("  - {} {}{}\n", gc, group.display_name(), dur));
            }
            for item in &section.items {
                let ic = match item.status { ItemStatus::Completed => "[x]", ItemStatus::Running => "[>]", _ => "[ ]" };
                md.push_str(&format!("  - {} {}\n", ic, item.name));
            }
        }
        md
    }
}
