use std::sync::Arc;

use axum::extract::Path;
use axum::{extract::Extension, http::StatusCode, response::IntoResponse, Json};

use crate::models::label::CreateLabel;
use crate::repositories::label_repository::LabelRepository;

use super::*;

pub async fn create_label<T: LabelRepository>(
    ValidatedJson(payload): ValidatedJson<CreateLabel>,
    Extension(repository): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let label = repository
        .create(payload.name)
        .await
        .or(Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    Ok((StatusCode::CREATED, Json(label)))
}

pub async fn all_label<T: LabelRepository>(
    Extension(repository): Extension<Arc<T>>,
) -> Result<impl IntoResponse, StatusCode> {
    let labels = repository.all().await.unwrap();
    Ok((StatusCode::OK, Json(labels)))
}

pub async fn delete_label<T: LabelRepository>(
    Path(id): Path<i32>,
    Extension(repository): Extension<Arc<T>>,
) -> StatusCode {
    repository
        .delete(id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .unwrap_or(StatusCode::NOT_FOUND)
}
