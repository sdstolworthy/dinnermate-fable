use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::code::generate_code;
use crate::error::{CoreError, RepoError};
use crate::model::{List, ListItem};
use crate::repo::ListRepo;
use crate::service::MAX_CODE_ATTEMPTS;

pub struct NewListItem {
    pub name: String,
    pub cuisine: Option<String>,
    pub price_level: Option<u8>,
    pub rating: Option<f32>,
    pub address: Option<String>,
    pub photo_url: Option<String>,
    pub source_restaurant_id: Option<String>,
}

pub struct ListService {
    repo: Arc<dyn ListRepo>,
}

impl ListService {
    pub fn new(repo: Arc<dyn ListRepo>) -> Self {
        Self { repo }
    }

    pub async fn create(&self, owner: Uuid, name: &str) -> Result<List, CoreError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::InvalidParams("list name must not be empty".to_string()));
        }
        for _ in 0..MAX_CODE_ATTEMPTS {
            let list = List {
                id: Uuid::new_v4(),
                code: generate_code(&mut rand::rng()),
                name: name.to_string(),
                owner_user_id: owner,
                created_at: Utc::now(),
            };
            match self.repo.create(&list).await {
                Ok(()) => return Ok(list),
                Err(RepoError::Conflict) => continue,
                Err(err) => return Err(err.into()),
            }
        }
        Err(CoreError::Repo(RepoError::Conflict))
    }

    pub async fn get(&self, code: &str) -> Result<(List, Vec<ListItem>), CoreError> {
        self.find_list(code).await
    }

    pub async fn add_item(
        &self,
        code: &str,
        user_id: Uuid,
        item: NewListItem,
    ) -> Result<ListItem, CoreError> {
        let name = item.name.trim();
        if name.is_empty() {
            return Err(CoreError::InvalidParams("item name must not be empty".to_string()));
        }
        let (list, _) = self.find_list(code).await?;
        let item = ListItem {
            id: Uuid::new_v4(),
            list_id: list.id,
            name: name.to_string(),
            cuisine: item.cuisine,
            price_level: item.price_level,
            rating: item.rating,
            address: item.address,
            photo_url: item.photo_url,
            added_by_user_id: user_id,
            source_restaurant_id: item.source_restaurant_id,
            created_at: Utc::now(),
        };
        self.repo.add_item(&item).await?;
        Ok(item)
    }

    pub async fn mine(&self, owner: Uuid) -> Result<Vec<List>, CoreError> {
        Ok(self.repo.lists_for_owner(owner).await?)
    }

    async fn find_list(&self, code: &str) -> Result<(List, Vec<ListItem>), CoreError> {
        self.repo
            .find_by_code(code)
            .await?
            .ok_or(CoreError::ListNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeListRepo;

    fn service() -> ListService {
        ListService::new(Arc::new(FakeListRepo::new()))
    }

    fn new_item(name: &str) -> NewListItem {
        NewListItem {
            name: name.into(),
            cuisine: Some("thai".into()),
            price_level: Some(2),
            rating: Some(4.5),
            address: None,
            photo_url: None,
            source_restaurant_id: Some("seed-001".into()),
        }
    }

    #[tokio::test]
    async fn create_and_get_roundtrip() {
        let service = service();
        let owner = Uuid::new_v4();
        let list = service.create(owner, "Date spots").await.unwrap();
        assert_eq!(list.code.len(), 6);
        let (got, items) = service.get(&list.code).await.unwrap();
        assert_eq!(got, list);
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn create_with_blank_name_is_invalid_params() {
        let service = service();
        for name in ["", "   "] {
            let err = service.create(Uuid::new_v4(), name).await.unwrap_err();
            assert!(matches!(err, CoreError::InvalidParams(_)), "name {name:?}: got {err:?}");
        }
    }

    #[tokio::test]
    async fn add_item_by_non_owner_succeeds() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Shared picks").await.unwrap();
        let other_user = Uuid::new_v4();
        let item = service.add_item(&list.code, other_user, new_item("Thai Garden")).await.unwrap();
        assert_eq!(item.added_by_user_id, other_user);
        let (_, items) = service.get(&list.code).await.unwrap();
        assert_eq!(items, vec![item]);
    }

    #[tokio::test]
    async fn add_item_with_blank_name_is_invalid_params() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Picks").await.unwrap();
        let err = service.add_item(&list.code, Uuid::new_v4(), new_item("  ")).await.unwrap_err();
        assert!(matches!(err, CoreError::InvalidParams(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn add_item_to_unknown_list_is_list_not_found() {
        let service = service();
        let err = service.add_item("NOSUCH", Uuid::new_v4(), new_item("Thai Garden")).await.unwrap_err();
        assert!(matches!(err, CoreError::ListNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn get_unknown_list_is_list_not_found() {
        let service = service();
        let err = service.get("NOSUCH").await.unwrap_err();
        assert!(matches!(err, CoreError::ListNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn mine_returns_only_owners_lists() {
        let service = service();
        let (alice, bob) = (Uuid::new_v4(), Uuid::new_v4());
        let alices = service.create(alice, "Alice's spots").await.unwrap();
        service.create(bob, "Bob's spots").await.unwrap();
        let mine = service.mine(alice).await.unwrap();
        assert_eq!(mine, vec![alices]);
    }
}
