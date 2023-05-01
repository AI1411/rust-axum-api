use axum::extract::{FromRequest, Path, RequestParts};
use axum::{
    async_trait, extract::Extension, http::StatusCode, response::IntoResponse, BoxError, Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::models::todo::{CreateTodo, UpdateTodo};

pub mod label_handler;
pub mod todo_handler;

#[derive(Debug)]
pub struct ValidatedJson<T>(T);

#[async_trait]
impl<T, B> FromRequest<B> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    B: http_body::Body + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req).await.map_err(|rejection| {
            let message = format!("json parse error: {}", rejection);
            (StatusCode::BAD_REQUEST, message)
        })?;
        value.validate().map_err(|rejection| {
            let message = format!("validation error: [{}]", rejection).replace('\n', ", ");
            (StatusCode::BAD_REQUEST, message)
        })?;
        Ok(ValidatedJson(value))
    }
}
