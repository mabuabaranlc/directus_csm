pub mod tabular;
pub mod cards;
pub mod calendar;
pub mod kanban;
pub mod map;


/// Available layout types
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutType {
    Tabular,
    Cards,
    Calendar,
    Kanban,
    Map,
}

impl LayoutType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "cards" => Self::Cards,
            "calendar" => Self::Calendar,
            "kanban" => Self::Kanban,
            "map" => Self::Map,
            _ => Self::Tabular,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Tabular => "reorder",
            Self::Cards => "grid_view",
            Self::Calendar => "event",
            Self::Kanban => "view_kanban",
            Self::Map => "map",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Tabular => "Table",
            Self::Cards => "Cards",
            Self::Calendar => "Calendar",
            Self::Kanban => "Kanban",
            Self::Map => "Map",
        }
    }
}
