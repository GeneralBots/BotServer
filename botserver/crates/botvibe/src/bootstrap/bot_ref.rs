use super::*;

impl BotRowRef {
    pub fn from_project(project: &Project) -> Self {
        Self {
            id: project.id,
            org_id: project.org_id,
            branch_id: project.branch_id,
            name: project.name.clone(),
        }
    }
}
