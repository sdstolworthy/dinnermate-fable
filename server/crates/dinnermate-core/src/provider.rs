use async_trait::async_trait;

use crate::error::ProviderError;
use crate::model::{Restaurant, RoomParams};

#[async_trait]
pub trait RestaurantProvider: Send + Sync {
    async fn search(&self, params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError>;
}
