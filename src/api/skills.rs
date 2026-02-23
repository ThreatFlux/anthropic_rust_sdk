//! Skills API implementation

use crate::{
    api::utils::build_path_with_query,
    client::{beta_headers, Client, API_VERSION},
    error::{AnthropicError, Result},
    models::skill::{
        Skill, SkillCreateRequest, SkillDeleteResponse, SkillFileUpload, SkillListParams,
        SkillListResponse, SkillVersion, SkillVersionCreateRequest, SkillVersionDeleteResponse,
        SkillVersionListParams, SkillVersionListResponse,
    },
    types::{HttpMethod, RequestOptions},
};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::{Form, Part},
};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, path::Path};

/// API client for Skills endpoints
#[derive(Clone)]
pub struct SkillsApi {
    client: Client,
}

impl SkillsApi {
    /// Create a new Skills API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Ensure requests to the Skills API include the required beta header.
    fn with_skills_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
        Some(options.unwrap_or_default().with_skills_api())
    }

    /// Build headers for multipart skill requests.
    fn build_skill_headers(&self, options: &Option<RequestOptions>) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        let auth_value = format!("Bearer {}", self.client.config().api_key);
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_value)
                .map_err(|e| AnthropicError::config(format!("Invalid auth header: {}", e)))?,
        );

        headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));

        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.client.config().user_agent)
                .map_err(|e| AnthropicError::config(format!("Invalid user agent: {}", e)))?,
        );

        let mut beta_features = vec![beta_headers::SKILLS_API];

        if let Some(options) = options {
            if options.enable_files_api {
                beta_features.push(beta_headers::FILES_API);
            }
            if options.enable_pdf_support {
                beta_features.push(beta_headers::PDF_SUPPORT);
            }
            if options.enable_prompt_caching {
                beta_features.push(beta_headers::PROMPT_CACHING);
            }
            if options.enable_1m_context {
                beta_features.push(beta_headers::CONTEXT_1M);
            }
            if options.enable_extended_thinking_tools {
                beta_features.push(beta_headers::EXTENDED_THINKING_TOOLS);
            }

            beta_features.extend(options.beta_features.iter().map(|s| s.as_str()));
        }

        let beta_header_value = beta_features.join(",");
        headers.insert(
            "anthropic-beta",
            HeaderValue::from_str(&beta_header_value)
                .map_err(|e| AnthropicError::config(format!("Invalid beta header: {}", e)))?,
        );

        if let Some(options) = options {
            for (key, value) in &options.headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| AnthropicError::config(format!("Invalid header name: {}", e)))?;
                headers.insert(
                    header_name,
                    HeaderValue::from_str(value).map_err(|e| {
                        AnthropicError::config(format!("Invalid header value: {}", e))
                    })?,
                );
            }
        }

        Ok(headers)
    }

    /// Build multipart form payload for skill upload operations.
    fn build_skill_upload_form(
        display_title: Option<&str>,
        files: Vec<SkillFileUpload>,
    ) -> Result<Form> {
        let mut form = Form::new();

        if let Some(display_title) = display_title {
            form = form.text("display_title", display_title.to_string());
        }

        for file in files {
            let part = Part::bytes(file.content)
                .file_name(file.filename)
                .mime_str(&file.mime_type)
                .map_err(|e| {
                    AnthropicError::file_error(format!("Invalid MIME type for skill file: {}", e))
                })?;
            form = form.part("files", part);
        }

        Ok(form)
    }

    /// Execute a multipart request against a skills endpoint.
    async fn multipart_request<T>(
        &self,
        method: HttpMethod,
        path: &str,
        form: Form,
        options: Option<RequestOptions>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut url = self.client.config().base_url.clone();
        url.set_path(&format!("/v1{}", path));

        let options = Self::with_skills_beta(options);
        let headers = self.build_skill_headers(&options)?;

        let request_client = reqwest::Client::new();
        let mut request_builder = match method {
            HttpMethod::Post => request_client.post(url),
            HttpMethod::Put => request_client.put(url),
            HttpMethod::Patch => request_client.patch(url),
            _ => {
                return Err(AnthropicError::invalid_input(
                    "Multipart skills requests only support POST, PUT, or PATCH",
                ))
            }
        }
        .headers(headers)
        .multipart(form);

        if let Some(timeout) = options.as_ref().and_then(|o| o.timeout) {
            request_builder = request_builder.timeout(timeout);
        }

        let response = request_builder.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| AnthropicError::json(e.to_string()))
    }

    /// Convert a local directory into skill upload files.
    fn collect_dir_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
        if !root.exists() {
            return Err(AnthropicError::file_error(format!(
                "Directory does not exist: {}",
                root.display()
            )));
        }
        if !root.is_dir() {
            return Err(AnthropicError::file_error(format!(
                "Path is not a directory: {}",
                root.display()
            )));
        }

        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir).map_err(|e| {
                AnthropicError::file_error(format!(
                    "Failed to read directory {}: {}",
                    dir.display(),
                    e
                ))
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| {
                    AnthropicError::file_error(format!("Failed to read directory entry: {}", e))
                })?;
                let path = entry.path();

                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    files.push(path);
                }
            }
        }

        files.sort();
        Ok(files)
    }

    /// Build skill upload files from a local directory.
    async fn build_upload_files_from_dir(root: &Path) -> Result<Vec<SkillFileUpload>> {
        let all_paths = Self::collect_dir_files(root)?;
        if all_paths.is_empty() {
            return Err(AnthropicError::invalid_input(format!(
                "No files found in directory: {}",
                root.display()
            )));
        }

        let root_name = root.file_name().ok_or_else(|| {
            AnthropicError::invalid_input(format!(
                "Skill directory path must have a final directory name: {}",
                root.display()
            ))
        })?;

        let mut files = Vec::with_capacity(all_paths.len());

        for path in all_paths {
            let rel = path.strip_prefix(root).map_err(|e| {
                AnthropicError::file_error(format!(
                    "Failed to compute relative path for {}: {}",
                    path.display(),
                    e
                ))
            })?;
            let remote_path = Path::new(root_name).join(rel);
            let remote_filename = remote_path.to_string_lossy().replace('\\', "/");
            let content = tokio::fs::read(&path).await.map_err(|e| {
                AnthropicError::file_error(format!("Failed to read file {}: {}", path.display(), e))
            })?;
            let mime_type = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();

            files.push(SkillFileUpload::new(remote_filename, content, mime_type));
        }

        Ok(files)
    }

    /// List skills
    pub async fn list(
        &self,
        params: Option<SkillListParams>,
        options: Option<RequestOptions>,
    ) -> Result<SkillListResponse> {
        let mut query_params = Vec::new();

        if let Some(params) = params {
            if let Some(limit) = params.limit {
                query_params.push(format!("limit={}", limit));
            }
            if let Some(page) = params.page {
                query_params.push(format!("page={}", page));
            }
            if let Some(source) = params.source {
                query_params.push(format!("source={}", source));
            }
        }

        let path = build_path_with_query("/skills", query_params);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await
    }

    /// List all skills by following pagination
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<Skill>> {
        let mut all_skills = Vec::new();
        let mut next_page: Option<String> = None;

        loop {
            let mut params = SkillListParams::new().with_limit(100);
            if let Some(page) = &next_page {
                params = params.with_page(page.clone());
            }

            let response = self.list(Some(params), options.clone()).await?;
            all_skills.extend(response.data);

            if !response.has_more {
                break;
            }

            if let Some(page) = response.next_page {
                if page.is_empty() {
                    break;
                }
                next_page = Some(page);
            } else {
                break;
            }
        }

        Ok(all_skills)
    }

    /// Retrieve a skill
    pub async fn get(&self, skill_id: &str, options: Option<RequestOptions>) -> Result<Skill> {
        let path = format!("/skills/{}", skill_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await
    }

    /// Create a skill by uploading skill files.
    pub async fn create(
        &self,
        request: SkillCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Skill> {
        request.validate()?;

        let form = Self::build_skill_upload_form(request.display_title.as_deref(), request.files)?;
        self.multipart_request(HttpMethod::Post, "/skills", form, options)
            .await
    }

    /// Create a skill directly from a local directory.
    pub async fn create_from_dir(
        &self,
        dir: impl AsRef<Path>,
        display_title: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<Skill> {
        let files = Self::build_upload_files_from_dir(dir.as_ref()).await?;
        let request = SkillCreateRequest::new();
        let request = files
            .into_iter()
            .fold(request, |req, file| req.add_file(file));
        let request = if let Some(title) = display_title {
            request.display_title(title)
        } else {
            request
        };

        self.create(request, options).await
    }

    /// Delete a skill
    pub async fn delete(
        &self,
        skill_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SkillDeleteResponse> {
        let path = format!("/skills/{}", skill_id);
        let response = self
            .client
            .request_stream(
                HttpMethod::Delete,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }

        let body = response.text().await.unwrap_or_default();
        if body.trim().is_empty() {
            return Ok(SkillDeleteResponse {
                id: skill_id.to_string(),
                object_type: Some("skill_deleted".to_string()),
                extra: HashMap::new(),
            });
        }

        serde_json::from_str(&body).map_err(|e| {
            AnthropicError::json(format!("Failed to parse delete skill response: {}", e))
        })
    }

    /// List versions for a specific skill.
    pub async fn list_versions(
        &self,
        skill_id: &str,
        params: Option<SkillVersionListParams>,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersionListResponse> {
        let mut query_params = Vec::new();

        if let Some(params) = params {
            if let Some(limit) = params.limit {
                query_params.push(format!("limit={}", limit));
            }
            if let Some(page) = params.page {
                query_params.push(format!("page={}", page));
            }
        }

        let path = build_path_with_query(&format!("/skills/{}/versions", skill_id), query_params);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await
    }

    /// List all versions for a specific skill by following pagination.
    pub async fn list_all_versions(
        &self,
        skill_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<SkillVersion>> {
        let mut all_versions = Vec::new();
        let mut next_page: Option<String> = None;

        loop {
            let mut params = SkillVersionListParams::new().with_limit(100);
            if let Some(page) = &next_page {
                params = params.with_page(page.clone());
            }

            let response = self
                .list_versions(skill_id, Some(params), options.clone())
                .await?;
            all_versions.extend(response.data);

            if !response.has_more {
                break;
            }

            if let Some(page) = response.next_page {
                if page.is_empty() {
                    break;
                }
                next_page = Some(page);
            } else {
                break;
            }
        }

        Ok(all_versions)
    }

    /// Get a specific skill version.
    pub async fn get_version(
        &self,
        skill_id: &str,
        version_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersion> {
        let path = format!("/skills/{}/versions/{}", skill_id, version_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await
    }

    /// Create a new version for an existing skill by uploading files.
    pub async fn create_version(
        &self,
        skill_id: &str,
        request: SkillVersionCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersion> {
        request.validate()?;

        let form = Self::build_skill_upload_form(None, request.files)?;
        self.multipart_request(
            HttpMethod::Post,
            &format!("/skills/{}/versions", skill_id),
            form,
            options,
        )
        .await
    }

    /// Convenience alias for creating a new version of a skill.
    ///
    /// Anthropic's API models updates as version creation.
    pub async fn update(
        &self,
        skill_id: &str,
        request: SkillVersionCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersion> {
        self.create_version(skill_id, request, options).await
    }

    /// Create a new skill version directly from a local directory.
    pub async fn create_version_from_dir(
        &self,
        skill_id: &str,
        dir: impl AsRef<Path>,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersion> {
        let files = Self::build_upload_files_from_dir(dir.as_ref()).await?;
        let request = SkillVersionCreateRequest::new();
        let request = files
            .into_iter()
            .fold(request, |req, file| req.add_file(file));
        self.create_version(skill_id, request, options).await
    }

    /// Delete a specific skill version.
    pub async fn delete_version(
        &self,
        skill_id: &str,
        version_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersionDeleteResponse> {
        let path = format!("/skills/{}/versions/{}", skill_id, version_id);
        let response = self
            .client
            .request_stream(
                HttpMethod::Delete,
                &path,
                None,
                Self::with_skills_beta(options),
            )
            .await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }

        let body = response.text().await.unwrap_or_default();
        if body.trim().is_empty() {
            return Ok(SkillVersionDeleteResponse {
                id: version_id.to_string(),
                object_type: Some("skill_version_deleted".to_string()),
                extra: HashMap::new(),
            });
        }

        serde_json::from_str(&body).map_err(|e| {
            AnthropicError::json(format!(
                "Failed to parse delete skill version response: {}",
                e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SkillsApi;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_build_upload_files_from_dir_preserves_root_dir_prefix() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("my_skill");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("SKILL.md"), "# My skill").unwrap();
        std::fs::write(root.join("docs").join("notes.txt"), "hello").unwrap();

        let files = SkillsApi::build_upload_files_from_dir(&root).await.unwrap();
        let names = files
            .iter()
            .map(|f| f.filename.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"my_skill/SKILL.md"));
        assert!(names.contains(&"my_skill/docs/notes.txt"));
    }
}
