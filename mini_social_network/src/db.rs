use std::sync::Arc;
use tokio_postgres::{Client, Error};
use crate::models::{User, Post};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    client: Arc<Client>,
}

impl Database {
    pub fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client)
        }
    }

    pub async fn init(&self) -> Result<(), Error> {
        self.client
            .batch_execute(
                "
                CREATE TABLE IF NOT EXISTS users (
                    id UUID PRIMARY KEY,
                    username TEXT UNIQUE NOT NULL,
                    password_hash TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS posts (
                    id UUID PRIMARY KEY,
                    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
                    content TEXT NOT NULL,
                    likes_count INTEGER DEFAULT 0,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
                );

                CREATE TABLE IF NOT EXISTS likes (
                    post_id UUID REFERENCES posts(id) ON DELETE CASCADE,
                    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
                    PRIMARY KEY (post_id, user_id)
                );
                ",
            )
            .await
    }

    pub async fn create_user(&self, user: &User) -> Result<bool, Error> {
        let result = self.client
            .execute(
                "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)",
                &[&user.id, &user.username, &user.password_hash],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Error> {
        let row = self.client
            .query_opt(
                "SELECT id, username, password_hash FROM users WHERE username = $1",
                &[&username],
            )
            .await?;

        Ok(row.map(|row| User {
            id: row.get(0),
            username: row.get(1),
            password_hash: row.get(2),
        }))
    }

    pub async fn create_post(&self, post: &Post) -> Result<bool, Error> {
        let result = self.client
            .execute(
                "INSERT INTO posts (id, user_id, content) VALUES ($1, $2, $3)",
                &[&post.id, &post.user_id, &post.content],
            )
            .await?;
        Ok(result > 0)
    }

    pub async fn get_post(&self, id: Uuid) -> Result<Option<Post>, Error> {
        let row = self.client
            .query_opt(
                "SELECT id, user_id, content, likes_count, created_at FROM posts WHERE id = $1",
                &[&id],
            )
            .await?;

        Ok(row.map(|row| Post {
            id: row.get(0),
            user_id: row.get(1),
            content: row.get(2),
            likes_count: row.get(3),
            created_at: row.get(4),
        }))
    }

    pub async fn delete_post(&self, id: Uuid, user_id: Uuid) -> Result<bool, Error> {
        if !self.check_post_ownership(id, user_id).await? {
            return Ok(false);
        }

        let result = self.client.execute(
            "DELETE FROM likes WHERE post_id = $1",
            &[&id],
        ).await?;

        Ok(result > 0)
    }

    pub async fn like_post(&self, post_id: Uuid, user_id: Uuid) -> Result<bool, Error> {
        if !self.check_post_exists(post_id).await? {
            return Ok(false);
        }

        if self.like_exists(post_id, user_id).await? {
            return Ok(false);
        }

        self.client.execute(
            "INSERT INTO likes (post_id, user_id) VALUES ($1, $2)",
            &[&post_id, &user_id],
        ).await?;

        Ok(true)
    }

    async fn check_post_ownership(&self, post_id: Uuid, user_id: Uuid) -> Result<bool, Error> {
        let result = self.client
            .query_opt(
                "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1 AND user_id = $2)",
                &[&post_id, &user_id],
            )
            .await?;

        Ok(result.map(|row| row.get::<_, bool>(0)).unwrap_or(false))
    }

    async fn check_post_exists(&self, post_id: Uuid) -> Result<bool, Error> {
        let result = self.client
            .query_opt(
                "SELECT EXISTS(SELECT 1 FROM posts WHERE id = $1)",
                &[&post_id],
            )
            .await?;

        Ok(result.map(|row| row.get::<_, bool>(0)).unwrap_or(false))
    }

    async fn like_exists(&self, post_id: Uuid, user_id: Uuid) -> Result<bool, Error> {
        let result = self.client
            .query_opt(
                "SELECT EXISTS(SELECT 1 FROM likes WHERE post_id = $1 AND user_id = $2)",
                &[&post_id, &user_id],
            )
            .await?;

        Ok(result.map(|row| row.get::<_, bool>(0)).unwrap_or(false))
    }
}