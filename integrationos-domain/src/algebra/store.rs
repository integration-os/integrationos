use crate::IntegrationOSError;
use crate::Store;
use bson::doc;
use futures::TryStreamExt;
use mongodb::bson::Document;
use mongodb::options::CountOptions;
use mongodb::{Collection, Database};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct MongoStore<T: Serialize + DeserializeOwned + Unpin + Sync + Send + Sync> {
    pub collection: Collection<T>,
}

impl<T: Serialize + DeserializeOwned + Unpin + Sync + Send + 'static> MongoStore<T> {
    pub async fn new(database: &Database, store: &Store) -> Result<Self, IntegrationOSError> {
        let collection = database.collection::<T>(store.to_string().as_str());
        Ok(Self { collection })
    }

    pub async fn aggregate(
        &self,
        pipeline: Vec<Document>,
    ) -> Result<Vec<Document>, IntegrationOSError> {
        let cursor = self.collection.aggregate(pipeline).await?;
        let results = cursor.try_collect().await?;
        Ok(results)
    }

    pub async fn get_one(&self, filter: Document) -> Result<Option<T>, IntegrationOSError> {
        Ok(self.collection.find_one(filter).await?)
    }

    pub async fn get_one_by_id(&self, id: &str) -> Result<Option<T>, IntegrationOSError> {
        let filter = doc! { "_id": id };

        Ok(self.collection.find_one(filter).await?)
    }

    pub async fn get_many(
        &self,
        filter: Option<Document>,
        selection: Option<Document>,
        sort: Option<Document>,
        limit: Option<u64>,
        skip: Option<u64>,
    ) -> Result<Vec<T>, IntegrationOSError> {
        let mut filter_options = mongodb::options::FindOptions::default();
        filter_options.sort = sort;
        filter_options.projection = selection;
        filter_options.limit = limit.map(|l| l as i64);
        filter_options.skip = skip;

        if filter_options.sort.is_none() {
            filter_options.sort = Some(doc! { "createdAt": -1 });
        }

        let cursor = self
            .collection
            .find(filter.unwrap_or_default())
            .with_options(filter_options)
            .await?;

        let records = cursor.try_collect().await?;

        Ok(records)
    }

    pub async fn create_one(&self, data: &T) -> Result<(), IntegrationOSError> {
        self.collection.insert_one(data).await?;

        Ok(())
    }

    pub async fn create_many(&self, data: &[T]) -> Result<(), IntegrationOSError> {
        self.collection.insert_many(data).await?;

        Ok(())
    }

    pub async fn update_one(&self, id: &str, data: Document) -> Result<(), IntegrationOSError> {
        let filter = doc! { "_id": id };

        self.collection.update_one(filter, data).await?;
        Ok(())
    }

    pub async fn update_many(
        &self,
        filter: Document,
        data: Document,
    ) -> Result<(), IntegrationOSError> {
        self.collection.update_many(filter, data).await?;

        Ok(())
    }

    pub async fn update_many_with_aggregation_pipeline(
        &self,
        filter: Document,
        data: &[Document],
    ) -> Result<(), IntegrationOSError> {
        self.collection.update_many(filter, data.to_vec()).await?;

        Ok(())
    }

    pub async fn count(
        &self,
        filter: Document,
        limit: Option<u64>,
    ) -> Result<u64, IntegrationOSError> {
        Ok(self
            .collection
            .count_documents(filter)
            .with_options(CountOptions::builder().limit(limit).build())
            .await?)
    }
}
