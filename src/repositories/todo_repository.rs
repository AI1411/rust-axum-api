use axum::async_trait;
use sqlx::PgPool;
use super::RepositoryError;
use crate::models::todo::{CreateTodo, Todo, UpdateTodo};

#[derive(Debug, Clone)]
pub struct TodoRepositoryForDb {
    pool: PgPool,
}

impl TodoRepositoryForDb {
    pub fn new(pool: PgPool) -> Self {
        TodoRepositoryForDb { pool }
    }
}

#[async_trait]
impl TodoRepository for TodoRepositoryForDb {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            INSERT INTO todos (text, completed)
            VALUES ($1, false)
            RETURNING *
            "#,
        )
            .bind(payload.text.clone())
            .fetch_one(&self.pool)
            .await?;

        Ok(todo)
    }

    async fn find(&self, id: i32) -> anyhow::Result<Todo> {
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            SELECT * FROM todos
            WHERE id = $1
            "#,
        )
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
                _ => RepositoryError::Unexpected(e.to_string()),
            })?;

        Ok(todo)
    }

    async fn all(&self) -> anyhow::Result<Vec<Todo>> {
        let todos = sqlx::query_as::<_, Todo>(
            r#"
            SELECT * FROM todos order by id desc
            "#,
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(todos)
    }

    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
        let old_todo = self.find(id).await?;
        let todo = sqlx::query_as::<_, Todo>(
            r#"
            UPDATE todos
            SET text=$1, completed=$2
            WHERE id = $3
            RETURNING *
            "#,
        )
            .bind(payload.text.unwrap_or(old_todo.text))
            .bind(payload.completed.unwrap_or(old_todo.completed))
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        Ok(todo)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM todos
            WHERE id = $1
            "#,
        )
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => RepositoryError::NotFound(id),
                _ => RepositoryError::Unexpected(e.to_string()),
            })?;

        Ok(())
    }
}

#[async_trait]
pub trait TodoRepository: Clone + std::marker::Send + std::marker::Sync + 'static {
    async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo>;
    async fn find(&self, id: i32) -> anyhow::Result<Todo>;
    async fn all(&self) -> anyhow::Result<Vec<Todo>>;
    async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[cfg(test)]
mod test {
    use std::env;

    use dotenv::dotenv;
    use sqlx::PgPool;

    use super::*;

    #[tokio::test]
    async fn crud_scenario() {
        dotenv().ok();
        let database_url = &env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(database_url)
            .await
            .expect(&format!("failed to connect database, url is [{}]", database_url));

        let repository = TodoRepositoryForDb::new(pool.clone());
        let todo_text = "test todo";

        // create
        let created = repository
            .create(CreateTodo::new(todo_text.to_string()))
            .await
            .expect("failed to create todo");
        assert_eq!(created.text, todo_text);
        assert_eq!(created.completed, false);

        // find
        let todo = repository
            .find(created.id)
            .await
            .expect("failed to find todo");
        assert_eq!(created, todo);

        // all
        let todos = repository
            .all()
            .await
            .expect("failed to find all todos");
        let todo = todos
            .first()
            .unwrap();
        assert_eq!(created, *todo);

        // update
        let updated_text = "updated todo";
        let updated = repository
            .update(created.id, UpdateTodo {
                text: Some(updated_text.to_string()),
                completed: Some(true),
            })
            .await
            .expect("failed to update todo");
        assert_eq!(created.id, todo.id);
        assert_eq!(updated.completed, true);

        // delete
        let _ = repository
            .delete(created.id)
            .await
            .expect("failed to delete todo");
        let _res = repository
            .find(created.id)
            .await;

        let todo_rows = sqlx::query(
            r#"
            SELECT * FROM todos where id = $1
            "#,
        )
            .bind(todo.id)
            .fetch_all(&pool)
            .await
            .expect("failed to fetch all todos");
        assert_eq!(todo_rows.len(), 0);
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    };

    use anyhow::Context;
    use axum::async_trait;

    use super::*;

    impl Todo {
        pub fn new(id: i32, text: String) -> Self {
            Self {
                id,
                text,
                completed: false,
            }
        }
    }

    impl CreateTodo {
        pub fn new(text: String) -> Self {
            Self { text }
        }
    }

    type TodoDatas = HashMap<i32, Todo>;

    #[derive(Debug, Clone)]
    pub struct TodoRepositoryForMemory {
        store: Arc<RwLock<TodoDatas>>,
    }

    impl TodoRepositoryForMemory {
        pub fn new() -> Self {
            TodoRepositoryForMemory {
                store: Arc::default(),
            }
        }

        fn write_store_ref(&self) -> RwLockWriteGuard<TodoDatas> {
            self.store.write().unwrap()
        }

        fn read_store_ref(&self) -> RwLockReadGuard<TodoDatas> {
            self.store.read().unwrap()
        }
    }

    #[async_trait]
    impl TodoRepository for TodoRepositoryForMemory {
        async fn create(&self, payload: CreateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_store_ref();
            let id = (store.len() + 1) as i32;
            let todo = Todo::new(id, payload.text.clone());
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn find(&self, id: i32) -> anyhow::Result<Todo> {
            let store = self.read_store_ref();
            let todo = store
                .get(&id)
                .map(|todo| todo.clone())
                .ok_or(RepositoryError::NotFound(id))?;
            Ok(todo)
        }

        async fn all(&self) -> anyhow::Result<Vec<Todo>> {
            let store = self.read_store_ref();
            Ok(Vec::from_iter(store.values().map(|todo| todo.clone())))
        }

        async fn update(&self, id: i32, payload: UpdateTodo) -> anyhow::Result<Todo> {
            let mut store = self.write_store_ref();
            let todo = store.get(&id).context(RepositoryError::NotFound(id))?;
            let text = payload.text.unwrap_or(todo.text.clone());
            let completed = payload.completed.unwrap_or(todo.completed);
            let todo = Todo {
                id,
                text,
                completed,
            };
            store.insert(id, todo.clone());
            Ok(todo)
        }

        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        #[tokio::test]
        async fn todo_crud_scenario() {
            let text = "todo text".to_string();
            let id = 1;
            let expected = Todo::new(id, text.clone());

            // create
            let repository = TodoRepositoryForMemory::new();
            let todo = repository
                .create(CreateTodo { text })
                .await
                .expect("failed create todo");
            assert_eq!(expected, todo);

            // find
            let todo = repository.find(todo.id).await.unwrap();
            assert_eq!(expected, todo);

            // all
            let todo = repository.all().await.expect("failed get all todo");
            assert_eq!(vec![expected], todo);

            // update
            let text = "update todo text".to_string();
            let todo = repository
                .update(
                    1,
                    UpdateTodo {
                        text: Some(text.clone()),
                        completed: Some(true),
                    },
                )
                .await
                .expect("failed update todo.");
            assert_eq!(
                Todo {
                    id,
                    text,
                    completed: true,
                },
                todo
            );

            // delete
            let res = repository.delete(id).await;
            assert!(res.is_ok())
        }
    }
}