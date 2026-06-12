use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::code::generate_code;
use crate::error::{CoreError, RepoError};
use crate::model::{List, ListItem, ListMembership};
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

    /// List detail plus the caller's `(is_member, is_owner)` flags. Reads stay
    /// open to anyone with the code; the flags drive client affordances.
    pub async fn get(
        &self,
        code: &str,
        user_id: Uuid,
    ) -> Result<(List, Vec<ListItem>, bool, bool), CoreError> {
        let (list, items) = self.find_list(code).await?;
        let is_member = self.repo.is_member(list.id, user_id).await?;
        let is_owner = list.owner_user_id == user_id;
        Ok((list, items, is_member, is_owner))
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
        if !self.repo.is_member(list.id, user_id).await? {
            return Err(CoreError::NotListMember);
        }
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

    pub async fn join(&self, code: &str, user_id: Uuid) -> Result<ListMembership, CoreError> {
        let (list, _) = self.find_list(code).await?;
        self.repo.join(list.id, user_id).await?;
        let is_owner = list.owner_user_id == user_id;
        Ok(ListMembership { list, is_owner })
    }

    pub async fn leave(&self, code: &str, user_id: Uuid) -> Result<(), CoreError> {
        let (list, _) = self.find_list(code).await?;
        if list.owner_user_id == user_id {
            return Err(CoreError::OwnerCannotLeave);
        }
        Ok(self.repo.leave(list.id, user_id).await?)
    }

    pub async fn mine(&self, user_id: Uuid) -> Result<Vec<ListMembership>, CoreError> {
        Ok(self.repo.lists_for_member(user_id).await?)
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
        let (got, items, is_member, is_owner) = service.get(&list.code, owner).await.unwrap();
        assert_eq!(got, list);
        assert!(items.is_empty());
        assert!(is_member && is_owner, "creator must be member and owner");
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
    async fn add_item_by_owner_succeeds() {
        let service = service();
        let owner = Uuid::new_v4();
        let list = service.create(owner, "Shared picks").await.unwrap();
        let item = service.add_item(&list.code, owner, new_item("Thai Garden")).await.unwrap();
        let (_, items, _, _) = service.get(&list.code, owner).await.unwrap();
        assert_eq!(items, vec![item]);
    }

    #[tokio::test]
    async fn add_item_by_joined_member_succeeds() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Shared picks").await.unwrap();
        let member = Uuid::new_v4();
        service.join(&list.code, member).await.unwrap();
        let item = service.add_item(&list.code, member, new_item("Thai Garden")).await.unwrap();
        assert_eq!(item.added_by_user_id, member);
    }

    #[tokio::test]
    async fn add_item_by_non_member_is_not_list_member() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Shared picks").await.unwrap();
        let err =
            service.add_item(&list.code, Uuid::new_v4(), new_item("Thai Garden")).await.unwrap_err();
        assert!(matches!(err, CoreError::NotListMember), "got {err:?}");
    }

    #[tokio::test]
    async fn join_is_idempotent_and_reports_ownership() {
        let service = service();
        let owner = Uuid::new_v4();
        let list = service.create(owner, "Shared picks").await.unwrap();
        let member = Uuid::new_v4();
        let first = service.join(&list.code, member).await.unwrap();
        let second = service.join(&list.code, member).await.unwrap();
        assert_eq!(first, ListMembership { list: list.clone(), is_owner: false });
        assert_eq!(second, first, "second join must be a no-op");
        let mine = service.mine(member).await.unwrap();
        assert_eq!(mine, vec![first], "duplicate join must not duplicate membership");
        let as_owner = service.join(&list.code, owner).await.unwrap();
        assert!(as_owner.is_owner);
    }

    #[tokio::test]
    async fn join_unknown_list_is_list_not_found() {
        let service = service();
        let err = service.join("NOSUCH", Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, CoreError::ListNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn leave_removes_membership() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Shared picks").await.unwrap();
        let member = Uuid::new_v4();
        service.join(&list.code, member).await.unwrap();
        service.leave(&list.code, member).await.unwrap();
        assert!(service.mine(member).await.unwrap().is_empty());
        let err = service.add_item(&list.code, member, new_item("Thai Garden")).await.unwrap_err();
        assert!(matches!(err, CoreError::NotListMember), "got {err:?}");
    }

    #[tokio::test]
    async fn leave_by_owner_is_owner_cannot_leave() {
        let service = service();
        let owner = Uuid::new_v4();
        let list = service.create(owner, "Shared picks").await.unwrap();
        let err = service.leave(&list.code, owner).await.unwrap_err();
        assert!(matches!(err, CoreError::OwnerCannotLeave), "got {err:?}");
        assert_eq!(service.mine(owner).await.unwrap().len(), 1, "owner must still be a member");
    }

    #[tokio::test]
    async fn leave_unknown_list_is_list_not_found() {
        let service = service();
        let err = service.leave("NOSUCH", Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, CoreError::ListNotFound), "got {err:?}");
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
        let err = service.get("NOSUCH", Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, CoreError::ListNotFound), "got {err:?}");
    }

    #[tokio::test]
    async fn get_reports_membership_flags_per_caller() {
        let service = service();
        let list = service.create(Uuid::new_v4(), "Flags").await.unwrap();
        let member = Uuid::new_v4();
        service.join(&list.code, member).await.unwrap();

        let (_, _, is_member, is_owner) = service.get(&list.code, member).await.unwrap();
        assert!(is_member && !is_owner, "joined non-owner");

        let (_, _, is_member, is_owner) = service.get(&list.code, Uuid::new_v4()).await.unwrap();
        assert!(!is_member && !is_owner, "stranger");
    }

    #[tokio::test]
    async fn mine_returns_owned_and_joined_with_flags_most_recent_first() {
        let service = service();
        let (alice, bob) = (Uuid::new_v4(), Uuid::new_v4());
        let alices = service.create(alice, "Alice's spots").await.unwrap();
        let bobs = service.create(bob, "Bob's spots").await.unwrap();
        service.join(&alices.code, bob).await.unwrap();

        let mine = service.mine(bob).await.unwrap();
        assert_eq!(
            mine,
            vec![
                ListMembership { list: alices, is_owner: false },
                ListMembership { list: bobs, is_owner: true },
            ],
            "joined most recently must come first"
        );

        let alices_view = service.mine(alice).await.unwrap();
        assert_eq!(alices_view.len(), 1);
        assert!(alices_view[0].is_owner);
    }
}
