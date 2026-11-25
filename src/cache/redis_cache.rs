use redis::AsyncCommands;

use crate::db::File;

#[derive(Clone)]
pub struct RedisCache {
    conn: redis::aio::ConnectionManager,
    ttl: u64, // secondes
}

impl RedisCache {
    pub async fn new(url: &str, ttl_secs: u64) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            conn,
            ttl: ttl_secs,
        })
    }

    fn cache_key(id: i64) -> String {
        format!("fichier:{id}")
    }

    pub async fn get(&self, id: i64) -> anyhow::Result<Option<File>> {
        let key = Self::cache_key(id);
        let val: Option<String> = self.conn.clone().get(&key).await?;

        match val {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn set(&mut self, file: &File) -> anyhow::Result<()> {
        let key = Self::cache_key(file.id as i64);
        let json = serde_json::to_string(file)?;
        self.conn.set_ex::<_, _, ()>(&key, json, self.ttl).await?;
        Ok(())
    }

    pub fn token_key(id: i64) -> String {
        format!("token:{id}")
    }

    pub async fn set_token(&mut self, user_id: i64, token: String) -> anyhow::Result<()> {
        let key = Self::token_key(user_id);
        self.conn.set_ex::<_, _, ()>(&key, token, self.ttl).await?;
        Ok(())
    }

    pub async fn invalidate(&mut self, id: i64) -> anyhow::Result<()> {
        self.conn.del::<_, ()>(Self::cache_key(id)).await?;
        Ok(())
    }
}
