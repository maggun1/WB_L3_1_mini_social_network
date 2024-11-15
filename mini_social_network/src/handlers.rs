use axum::{
    extract::{State, Path, Json},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;
use crate::{
    db::Database,
    models::{User, Post, RegisterRequest, LoginRequest, CreatePostRequest},
    auth::{create_jwt, verify_jwt},
};
use serde_json::json;

pub async fn register(
    State(db): State<Database>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let password_hash = hash(req.password.as_bytes(), DEFAULT_COST)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to hash password"}))
        ))?;

    let user = User {
        id: Uuid::new_v4(),
        username: req.username,
        password_hash,
    };

    let result = db.create_user(&user).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?;

    if !result {
        return Err((StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))
    }

    let token = create_jwt(user.id);
    Ok((StatusCode::CREATED, Json(json!({ "token": token }))))
}

pub async fn login(
    State(db): State<Database>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user = db.get_user_by_username(&req.username).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid credentials"}))
        ))?;

    if !verify(req.password.as_bytes(), &user.password_hash)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to verify password"}))
        ))? {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid credentials"}))
        ));
    }

    let token = create_jwt(user.id);
    Ok(Json(json!({ "token": token })))
}

fn extract_user_id(headers: &HeaderMap) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing authorization header"}))
        ))?;

    verify_jwt(token).ok_or_else(|| (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Invalid token"}))
    ))
}

pub async fn create_post(
    State(db): State<Database>,
    headers: HeaderMap,
    Json(req): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&headers)?;

    let post = Post {
        id: Uuid::new_v4(),
        user_id,
        content: req.content,
        likes_count: 0,
        created_at: chrono::Utc::now(),
    };

    let result = db.create_post(&post).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?;

    if !result {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ));
    }

    Ok((StatusCode::CREATED, Json(json!(post))))
}

pub async fn get_post(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let post = db.get_post(id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Post not found"}))
        ))?;

    Ok(Json(post))
}

pub async fn delete_post(
    State(db): State<Database>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&headers)?;

    let result = db.delete_post(id, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?;

    if !result {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Post not found or unauthorized"}))
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn like_post(
    State(db): State<Database>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = extract_user_id(&headers)?;

    let result = db.like_post(id, user_id).await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database error"}))
        ))?;

    if !result {
            return Err((
            StatusCode::NOT_ACCEPTABLE,
            Json(json!({"error": "Post not found or already liked"}))
        ));
    }
    Ok(StatusCode::OK)
}