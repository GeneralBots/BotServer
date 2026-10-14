use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagePermissionType {
    View,
    Edit,
    Comment,
    Share,
    Delete,
}

pub fn duplicate_page(
    page: &Page,
    new_parent_id: Option<Uuid>,
    new_workspace_id: Option<Uuid>,
    new_title: Option<String>,
    created_by: Uuid,
    pages: &HashMap<Uuid, Page>,
    include_children: bool,
) -> Vec<Page> {
    let mut duplicated_pages = Vec::new();
    let now = Utc::now();

    let new_page = Page {
        id: Uuid::new_v4(),
        workspace_id: new_workspace_id.unwrap_or(page.workspace_id),
        parent_id: new_parent_id,
        title: new_title.unwrap_or_else(|| format!("{} (Copy)", page.title)),
        icon: page.icon.clone(),
        cover_image: page.cover_image.clone(),
        blocks: page.blocks.clone(),
        children: Vec::new(),
        properties: page.properties.clone(),
        permissions: PagePermissions::default(),
        is_template: false,
        template_id: page.template_id,
        created_at: now,
        updated_at: now,
        created_by,
        last_edited_by: created_by,
    };

    let new_page_id = new_page.id;
    duplicated_pages.push(new_page);

    if include_children {
        for child_id in &page.children {
            if let Some(child_page) = pages.get(child_id) {
                let child_duplicates = duplicate_page(
                    child_page,
                    Some(new_page_id),
                    new_workspace_id,
                    None,
                    created_by,
                    pages,
                    true,
                );
                duplicated_pages.extend(child_duplicates);
            }
        }
    }

    duplicated_pages
}

pub fn sort_pages_by_title(pages: &mut [PageSummary]) {
    pages.sort_by_key(|a| a.title.to_lowercase());
}

pub fn sort_pages_by_updated(pages: &mut [PageSummary]) {
    pages.sort_by_key(|a| std::cmp::Reverse(a.updated_at.clone()));
}

pub fn sort_pages_by_created(pages: &mut [PageSummary]) {
    pages.sort_by_key(|a| std::cmp::Reverse(a.created_at.clone()));
}

pub fn filter_pages_by_date_range(
    pages: Vec<PageSummary>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Vec<PageSummary> {
    pages
        .into_iter()
        .filter(|p| {
            let after_start = start.map(|s| p.updated_at >= s).unwrap_or(true);
            let before_end = end.map(|e| p.updated_at <= e).unwrap_or(true);
            after_start && before_end
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStats {
    pub total_blocks: usize,
    pub total_words: usize,
    pub total_characters: usize,
    pub has_images: bool,
    pub has_tables: bool,
    pub has_code: bool,
    pub child_count: usize,
    pub comment_count: usize,
}

pub fn calculate_page_stats(page: &Page, comment_count: usize) -> PageStats {
    let mut stats = PageStats {
        total_blocks: 0,
        total_words: 0,
        total_characters: 0,
        has_images: false,
        has_tables: false,
        has_code: false,
        child_count: page.children.len(),
        comment_count,
    };

    count_blocks_stats(&page.blocks, &mut stats);

    stats
}

pub(crate) fn count_blocks_stats(blocks: &[Block], stats: &mut PageStats) {
    for block in blocks {
        stats.total_blocks += 1;

        match block.block_type {
            BlockType::Image => stats.has_images = true,
            BlockType::Table => stats.has_tables = true,
            BlockType::Code => stats.has_code = true,
            _ => {}
        }

        if let BlockContent::Text { text: rich_text } = &block.content {
            for segment in &rich_text.segments {
                stats.total_characters += segment.text.len();
                stats.total_words += segment.text.split_whitespace().count();
            }
        }

        count_blocks_stats(&block.children, stats);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub blocks: Vec<Block>,
    pub properties: HashMap<String, TemplateProperty>,
    pub category: TemplateCategory,
    pub tags: Vec<String>,
    pub is_system: bool,
    pub organization_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub use_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateProperty {
    pub name: String,
    pub property_type: PropertyType,
    pub default_value: Option<serde_json::Value>,
    pub required: bool,
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PropertyType {
    Text,
    Number,
    Date,
    Select,
    MultiSelect,
    Checkbox,
    Url,
    Email,
    Person,
    Files,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    Meeting,
    Project,
    Documentation,
    Planning,
    Personal,
    Team,
    Marketing,
    Engineering,
    Sales,
    Hr,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub settings: WorkspaceSettings,
    pub page_templates: Vec<PageTemplateRef>,
    pub default_structure: Vec<PageStructure>,
    pub category: TemplateCategory,
    pub is_system: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTemplateRef {
    pub template_id: Uuid,
    pub position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStructure {
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub template_id: Option<Uuid>,
    pub children: Vec<PageStructure>,
}

#[derive(Debug, Clone)]
pub enum TemplateError {
    TemplateNotFound,
    CannotModifySystemTemplate,
    InvalidTemplate(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemplateNotFound => write!(f, "Template not found"),
            Self::CannotModifySystemTemplate => write!(f, "Cannot modify system template"),
            Self::InvalidTemplate(e) => write!(f, "Invalid template: {e}"),
        }
    }
}

impl std::error::Error for TemplateError {}

pub fn clone_blocks_with_new_ids(blocks: &[Block], created_by: Uuid) -> Vec<Block> {
    let now = Utc::now();
    blocks
        .iter()
        .map(|block| {
            let mut new_block = block.clone();
            new_block.id = Uuid::new_v4();
            new_block.created_at = now;
            new_block.updated_at = now;
            new_block.created_by = created_by;
            new_block.children = clone_blocks_with_new_ids(&block.children, created_by);
            new_block
        })
        .collect()
}

pub fn apply_template_to_page(
    template: &PageTemplate,
    workspace_id: Uuid,
    parent_id: Option<Uuid>,
    title: Option<String>,
    created_by: Uuid,
) -> Page {
    let now = Utc::now();
    Page {
        id: Uuid::new_v4(),
        workspace_id,
        parent_id,
        title: title.unwrap_or_else(|| template.name.clone()),
        icon: template.icon.clone(),
        cover_image: template.cover_image.clone(),
        blocks: clone_blocks_with_new_ids(&template.blocks, created_by),
        children: Vec::new(),
        properties: HashMap::new(),
        permissions: PagePermissions::default(),
        is_template: false,
        template_id: Some(template.id),
        created_at: now,
        updated_at: now,
        created_by,
        last_edited_by: created_by,
    }
}

pub fn get_system_templates() -> Vec<PageTemplate> {
    let system_user = Uuid::nil();
    let now = Utc::now();

    vec![
        PageTemplate {
            id: Uuid::new_v4(),
            name: "Meeting Notes".to_string(),
            description: "Template for meeting notes with agenda and action items".to_string(),
            icon: None,
            cover_image: None,
            blocks: vec![],
            properties: HashMap::new(),
            category: TemplateCategory::Meeting,
            tags: vec!["meeting".to_string(), "notes".to_string()],
            is_system: true,
            organization_id: None,
            workspace_id: None,
            created_by: system_user,
            created_at: now,
            updated_at: now,
            use_count: 0,
        },
        PageTemplate {
            id: Uuid::new_v4(),
            name: "Project Brief".to_string(),
            description: "Template for project briefs and planning".to_string(),
            icon: None,
            cover_image: None,
            blocks: vec![],
            properties: HashMap::new(),
            category: TemplateCategory::Project,
            tags: vec!["project".to_string(), "planning".to_string()],
            is_system: true,
            organization_id: None,
            workspace_id: None,
            created_by: system_user,
            created_at: now,
            updated_at: now,
            use_count: 0,
        },
        PageTemplate {
            id: Uuid::new_v4(),
            name: "Documentation".to_string(),
            description: "Template for technical documentation".to_string(),
            icon: None,
            cover_image: None,
            blocks: vec![],
            properties: HashMap::new(),
            category: TemplateCategory::Documentation,
            tags: vec!["docs".to_string(), "technical".to_string()],
            is_system: true,
            organization_id: None,
            workspace_id: None,
            created_by: system_user,
            created_at: now,
            updated_at: now,
            use_count: 0,
        },
    ]
}

