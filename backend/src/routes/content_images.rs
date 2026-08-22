//! 本地课程实物图片：登录用户读取，管理员上传或明确确认删除。

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Extension, Multipart, Path, Query, State};
use axum::http::{header, Response, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::SharedState;

const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/content-images/{kind}/{id}", get(read_image))
        .route("/api/admin/content-images", post(upload_image))
        .route(
            "/api/admin/content-images/{kind}/{id}",
            delete(delete_image),
        )
        .layer(DefaultBodyLimit::max(MAX_IMAGE_BYTES + 64 * 1024))
}

async fn read_image(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path((kind, id)): Path<(String, String)>,
) -> AppResult<Response<Body>> {
    validate_target(&kind, &id)?;
    if user.role != "admin" && !target_is_published(&state, &kind, &id).await? {
        return Ok(empty_not_found());
    }
    let Some((path, format)) = find_image(&state.cfg.content_image_dir, &kind, &id) else {
        return Ok(empty_not_found());
    };
    let bytes = std::fs::read(path)?;
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(format.mime()),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, max-age=300"),
    );
    Ok(response)
}

async fn upload_image(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    let mut kind = None;
    let mut target_id = None;
    let mut confirmation = None;
    let mut image = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("无法读取图片表单".into()))?
    {
        match field.name().unwrap_or_default() {
            "kind" => kind = field.text().await.ok(),
            "target_id" => target_id = field.text().await.ok(),
            "confirmation" => confirmation = field.text().await.ok(),
            "image" => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::BadRequest("无法读取图片".into()))?;
                if bytes.len() > MAX_IMAGE_BYTES {
                    return Err(AppError::BadRequest("图片不能超过 5MB".into()));
                }
                image = Some(bytes.to_vec());
            }
            _ => {}
        }
    }
    let kind = kind.ok_or_else(|| AppError::BadRequest("缺少 kind".into()))?;
    let target_id = target_id.ok_or_else(|| AppError::BadRequest("缺少 target_id".into()))?;
    validate_target(&kind, &target_id)?;
    require_target_exists(&state, &kind, &target_id).await?;
    let bytes = image.ok_or_else(|| AppError::BadRequest("缺少 image".into()))?;
    let format = detect_image_format(&bytes)
        .ok_or_else(|| AppError::BadRequest("只支持 JPEG、PNG 或 WebP 实物照片".into()))?;
    let existing = find_image(&state.cfg.content_image_dir, &kind, &target_id);
    if existing.is_some() && confirmation.as_deref() != Some("REPLACE_CONTENT_IMAGE") {
        return Err(AppError::BadRequest("替换已有图片前需要明确确认".into()));
    }

    let directory = std::path::Path::new(&state.cfg.content_image_dir).join(&kind);
    std::fs::create_dir_all(&directory)?;
    let final_path = directory.join(format!("{}.{}", target_id, format.extension()));
    let temporary_path = directory.join(format!(".{}.{}.tmp", target_id, Uuid::new_v4()));
    std::fs::write(&temporary_path, &bytes)?;
    if let Err(error) = std::fs::rename(&temporary_path, &final_path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(error.into());
    }
    if let Some((old_path, _)) = existing {
        if old_path != final_path {
            if let Err(error) = std::fs::remove_file(&old_path) {
                tracing::warn!(path = %old_path.display(), error = %error, "旧课程图片清理失败");
            }
        }
    }
    Ok(Json(json!({
        "ok": true,
        "kind": kind,
        "target_id": target_id,
        "content_type": format.mime(),
        "byte_length": bytes.len(),
        "version": chrono::Utc::now().timestamp(),
    })))
}

#[derive(Deserialize)]
struct DeleteConfirmation {
    confirmation: String,
}

async fn delete_image(
    State(state): State<SharedState>,
    Extension(user): Extension<AuthUser>,
    Path((kind, id)): Path<(String, String)>,
    Query(query): Query<DeleteConfirmation>,
) -> AppResult<Json<serde_json::Value>> {
    require_admin(&user)?;
    validate_target(&kind, &id)?;
    if query.confirmation != "DELETE_CONTENT_IMAGE" {
        return Err(AppError::BadRequest("删除确认文本不匹配".into()));
    }
    let Some((path, _)) = find_image(&state.cfg.content_image_dir, &kind, &id) else {
        return Err(AppError::NotFound("课程图片不存在".into()));
    };
    std::fs::remove_file(path)?;
    Ok(Json(json!({ "ok": true, "kind": kind, "target_id": id })))
}

fn require_admin(user: &AuthUser) -> AppResult<()> {
    if user.role == "admin" {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限".into()))
    }
}

async fn require_target_exists(state: &SharedState, kind: &str, id: &str) -> AppResult<()> {
    let exists: bool = match kind {
        "word" => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM word WHERE id=?)")
                .bind(id)
                .fetch_one(&state.pool)
                .await?
        }
        "activity" => {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM subject_item WHERE id=?)")
                .bind(id)
                .fetch_one(&state.pool)
                .await?
        }
        _ => false,
    };
    if exists {
        Ok(())
    } else {
        Err(AppError::NotFound("课程不存在".into()))
    }
}

async fn target_is_published(state: &SharedState, kind: &str, id: &str) -> AppResult<bool> {
    let published = match kind {
        "word" => {
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM word WHERE id=? AND review_status='published')",
            )
            .bind(id)
            .fetch_one(&state.pool)
            .await?
        }
        "activity" => sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM subject_item WHERE id=? AND review_status='published')",
        )
        .bind(id)
        .fetch_one(&state.pool)
        .await?,
        _ => false,
    };
    Ok(published)
}

fn validate_target(kind: &str, id: &str) -> AppResult<()> {
    if !["word", "activity"].contains(&kind)
        || id.is_empty()
        || id.len() > 80
        || !id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err(AppError::BadRequest("图片课程标识非法".into()));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ImageFormat {
    Jpeg,
    Png,
    WebP,
}

impl ImageFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
        }
    }

    fn mime(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
        }
    }
}

fn detect_image_format(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(ImageFormat::Jpeg)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else {
        None
    }
}

fn find_image(root: &str, kind: &str, id: &str) -> Option<(std::path::PathBuf, ImageFormat)> {
    for format in [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::WebP] {
        let path =
            std::path::Path::new(root)
                .join(kind)
                .join(format!("{}.{}", id, format.extension()));
        if path.is_file() {
            return Some((path, format));
        }
    }
    None
}

/// 管理后台课程列表只需要知道是否已有照片，不暴露文件路径。
pub(crate) fn image_exists(root: &str, kind: &str, id: &str) -> bool {
    validate_target(kind, id).is_ok() && find_image(root, kind, id).is_some()
}

fn empty_not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .expect("static response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_safe_course_identifiers() {
        assert!(validate_target("word", "word_cup-1").is_ok());
        assert!(validate_target("activity", "math_shape_1").is_ok());
        assert!(validate_target("word", "../auth.json").is_err());
        assert!(validate_target("sentence", "sent_one").is_err());
    }

    #[test]
    fn detects_allowed_raster_formats_and_rejects_svg() {
        assert_eq!(
            detect_image_format(&[0xff, 0xd8, 0xff, 0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            detect_image_format(b"\x89PNG\r\n\x1a\nrest"),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            detect_image_format(b"RIFF1234WEBPrest"),
            Some(ImageFormat::WebP)
        );
        assert_eq!(detect_image_format(b"<svg onload='alert(1)'>"), None);
    }

    #[test]
    fn image_exists_never_resolves_unsafe_paths() {
        assert!(!image_exists("/tmp", "word", "../auth.json"));
        assert!(!image_exists("/tmp", "sentence", "sentence_one"));
        assert!(!image_exists("/tmp", "word", "word_missing"));
    }
}
