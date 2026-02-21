use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Section {
    Sp,
    I,
    Si,
    P,
}

impl Section {
    pub fn all() -> &'static [Section] {
        &[Section::Sp, Section::I, Section::Si, Section::P]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Section::Sp => "Sp",
            Section::I => "I",
            Section::Si => "Si",
            Section::P => "P",
        }
    }

    pub fn parse(s: &str) -> Option<Section> {
        match s {
            "Sp" => Some(Section::Sp),
            "I" => Some(Section::I),
            "Si" => Some(Section::Si),
            "P" => Some(Section::P),
            _ => None,
        }
    }
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl Importance {
    pub fn all() -> &'static [Importance] {
        &[Importance::Low, Importance::Medium, Importance::High, Importance::Critical]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Importance::Low => "low",
            Importance::Medium => "medium",
            Importance::High => "high",
            Importance::Critical => "critical",
        }
    }

    pub fn parse(s: &str) -> Option<Importance> {
        match s {
            "low" => Some(Importance::Low),
            "medium" => Some(Importance::Medium),
            "high" => Some(Importance::High),
            "critical" => Some(Importance::Critical),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Importance::Low => "Low",
            Importance::Medium => "Medium",
            Importance::High => "High",
            Importance::Critical => "Critical",
        }
    }
}

impl std::fmt::Display for Importance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub section: Section,
    pub title: String,
    pub completed: bool,
    pub importance: Importance,
    /// Optional due date in YYYY-MM-DD format
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTodoRequest {
    pub section: Section,
    pub title: String,
    #[serde(default)]
    pub importance: Option<Importance>,
    #[serde(default)]
    pub due_date: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UpdateTodoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<Section>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<Importance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_date: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionCount {
    pub section: Section,
    pub total: usize,
    pub completed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub deleted: String,
}
