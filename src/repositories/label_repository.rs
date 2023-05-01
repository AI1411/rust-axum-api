use axum::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::label::*;

use super::RepositoryError;

#[async_trait]
pub trait LabelRepository: Clone + Send + Sync + 'static {
    async fn create(&self, name: String) -> anyhow::Result<Label>;
    async fn all(&self) -> anyhow::Result<Vec<Label>>;
    async fn delete(&self, id: i32) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct LabelRepositoryForDB {
    pool: PgPool,
}

impl LabelRepositoryForDB {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LabelRepository for LabelRepositoryForDB {
    async fn create(&self, name: String) -> anyhow::Result<Label> {
        let optional_label = sqlx::query_as::<_, Label>(
            r#"
                select * from labels where name = $1
                 "#,
        )
        .bind(name.clone())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(label) = optional_label {
            return Err(RepositoryError::Duplicate(label.id).into());
        }

        let label = sqlx::query_as::<_, Label>(
            r#"
                insert into labels ( name )
                values ( $1 )
                returning *
                "#,
        )
        .bind(name.clone())
        .fetch_one(&self.pool)
        .await?;

        Ok(label)
    }

    async fn all(&self) -> anyhow::Result<Vec<Label>> {
        let labels = sqlx::query_as::<_, Label>(
            r#"
            SELECT * FROM labels order by labels.id asc
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(labels)
    }

    async fn delete(&self, id: i32) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            DELETE FROM labels
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repositories::label_repository::test_utils::LabelRepositoryForMemory;

    #[tokio::test]
    async fn crud_scenario() {
        let repository = LabelRepositoryForMemory::new();
        let label_text = "test_label";

        // create
        let label = repository
            .create(label_text.to_string())
            .await
            .expect("[create] returned Err");
        assert_eq!(label.name, label_text);

        // all
        let labels = repository.all().await.expect("[all] returned Err");
        let label = labels.last().unwrap();
        assert_eq!(label.name, label_text);

        // delete
        repository
            .delete(label.id)
            .await
            .expect("[delete] returned Err");
    }
}

#[cfg(test)]
pub mod test_utils {
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

    use axum::async_trait;

    use crate::models::label::CreateLabel;
    use crate::repositories::label_repository::LabelRepository;
    use crate::repositories::RepositoryError;

    use super::Label;

    impl Label {
        pub fn new(id: i32, name: String) -> Self {
            Self { id, name }
        }
    }

    type LabelData = HashMap<i32, Label>;

    #[derive(Debug, Clone)]
    pub struct LabelRepositoryForMemory {
        data: Arc<RwLock<LabelData>>,
    }

    impl LabelRepositoryForMemory {
        pub fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        pub fn read_store_ref(&self) -> RwLockReadGuard<LabelData> {
            self.data.read().unwrap()
        }

        pub fn write_store_ref(&self) -> RwLockWriteGuard<LabelData> {
            self.data.write().unwrap()
        }
    }

    #[async_trait]
    impl LabelRepository for LabelRepositoryForMemory {
        async fn create(&self, name: String) -> anyhow::Result<Label> {
            let mut store = self.write_store_ref();
            if let Some((_key, label)) = store.iter().find(|(_key, label)| label.name == name) {
                return Ok(label.clone());
            };

            let id = (store.len() + 1) as i32;
            let label = Label::new(id, name.clone());
            store.insert(id, label.clone());
            Ok(label)
        }

        async fn all(&self) -> anyhow::Result<Vec<Label>> {
            let store = self.read_store_ref();
            let labels = Vec::from_iter(store.values().map(|label| label.clone()));
            Ok(labels)
        }

        async fn delete(&self, id: i32) -> anyhow::Result<()> {
            let mut store = self.write_store_ref();
            store.remove(&id).ok_or(RepositoryError::NotFound(id))?;
            Ok(())
        }
    }

    mod test {
        use std::vec;

        use crate::models::label::Label;

        use super::{LabelRepository, LabelRepositoryForMemory};

        #[tokio::test]
        async fn label_crud_scenario() {
            let text = "label text".to_string();
            let id = 1;
            let expected = Label::new(id, text.clone());

            // create
            let repository = LabelRepositoryForMemory::new();
            let label = repository
                .create(text.clone())
                .await
                .expect("failed label create");
            assert_eq!(expected, label);

            // all
            let label = repository.all().await.unwrap();
            assert_eq!(vec![expected], label);

            // delete
            let res = repository.delete(id).await;
            assert!(res.is_ok())
        }
    }
}
