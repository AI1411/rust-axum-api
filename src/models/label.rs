use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Label {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Validate)]
pub struct CreateLabel {
    #[validate(length(min = 1, message = "name is required"))]
    #[validate(length(max = 255, message = "name is too long"))]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateLabel {
    pub id: i32,
    pub name: String,
}
