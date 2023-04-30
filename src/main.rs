use std::{env, sync::Arc};
use std::net::SocketAddr;

use axum::{
    extract::Extension,
    Router,
    routing::{get, post},
};

use handlers::{all_todo, create_todo, delete_todo, find_todo, update_todo};

use crate::repositories::{TodoRepository, TodoRepositoryForMemory};

mod handlers;
mod repositories;

#[tokio::main]
async fn main() {
    // logging
    let log_level = env::var("RUST_LOG").unwrap_or("info".to_string());
    env::set_var("RUST_LOG", log_level);
    tracing_subscriber::fmt::init();

    let repository = TodoRepositoryForMemory::new();
    let app = create_app(repository);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn create_app<T: TodoRepository>(repository: T) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/todos", post(create_todo::<T>).get(all_todo::<T>))
        .route(
            "/todos/:id",
            get(find_todo::<T>)
                .patch(update_todo::<T>)
                .delete(delete_todo::<T>),
        )
        .layer(Extension(Arc::new(repository)))
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[cfg(test)]
mod test {
    use axum::{
        body::Body,
        http::{header, Method, Request},
    };
    use axum::response::Response;
    use hyper::StatusCode;
    use tower::ServiceExt;

    use crate::repositories::{CreateTodo, Todo};

    use super::*;

    fn build_todo_req_with_json(path: &str, method: Method, json_body: String) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(path)
            .header(header::CONTENT_TYPE, mime::APPLICATION_JSON.to_string())
            .body(Body::from(json_body))
            .unwrap()
    }

    fn build_todo_req_with_empty(method: Method, path: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap()
    }

    async fn res_to_todo(res: Response) -> Todo {
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body: String = String::from_utf8(bytes.to_vec()).unwrap();
        let todo: Todo = serde_json::from_str(&body)
            .expect(&format!("cannot convert Todo. body: {}", body));
        todo
    }

    #[tokio::test]
    async fn should_created_todo() {
        let expected = Todo::new(1, "test".to_string());

        let repository = TodoRepositoryForMemory::new();
        let req = build_todo_req_with_json(
            "/todos",
            Method::POST,
            r#"{"text":"test"}"#.to_string(),
        );
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_find_todo() {
        let expected = Todo::new(1, "test".to_string());

        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("test".to_string())).await.expect("cannot create todo");
        let req = build_todo_req_with_empty(Method::GET, "/todos/1");
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_get_all_todos() {
        let expected = Todo::new(1, "test".to_string());

        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("test".to_string())).await.expect("cannot create todo");
        let req = build_todo_req_with_empty(Method::GET, "/todos");
        let res = create_app(repository).oneshot(req).await.unwrap();
        let bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        let body: String = String::from_utf8(bytes.to_vec()).unwrap();
        let todos: Vec<Todo> = serde_json::from_str(&body).expect(&format!("cannot convert Todo. body: {}", body));
        assert_eq!(todos[0], expected);
    }

    #[tokio::test]
    async fn should_update_todo() {
        let expected = Todo::new(1, "updated".to_string());

        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("test".to_string())).await.expect("cannot create todo");
        let req = build_todo_req_with_json(
            "/todos/1",
            Method::PATCH,
            r#"{"id": 1,"text":"updated", "completed": false}"#.to_string(),
        );
        let res = create_app(repository).oneshot(req).await.unwrap();
        let todo = res_to_todo(res).await;
        assert_eq!(todo, expected);
    }

    #[tokio::test]
    async fn should_delete_todo() {
        let repository = TodoRepositoryForMemory::new();
        repository.create(CreateTodo::new("test".to_string())).await.expect("cannot create todo");
        let req = build_todo_req_with_empty(Method::DELETE, "/todos/1");
        let res = create_app(repository).oneshot(req).await.unwrap();
        assert_eq!(StatusCode::NO_CONTENT, res.status());
    }
}
