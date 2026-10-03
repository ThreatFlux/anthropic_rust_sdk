//! Skills API implementation

use crate::{
    api::utils::{build_path_with_query, encode_query_value, paginate, TraversalPage},
    client::{beta_headers, Client},
    error::{AnthropicError, Result},
    models::skill::{
        CurrentSkill, CurrentSkillCreateRequest, CurrentSkillListResponse, CurrentSkillVersion,
        CurrentSkillVersionCreateRequest, CurrentSkillVersionListResponse, Skill,
        SkillCreateRequest, SkillDeleteResponse, SkillFileUpload, SkillListParams,
        SkillListResponse, SkillVersion, SkillVersionCreateRequest, SkillVersionDeleteResponse,
        SkillVersionListParams, SkillVersionListResponse,
    },
    types::{HttpMethod, PageStream, PaginationLimits, RequestOptions},
};
use reqwest::multipart::{Form, Part};
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
        self.client
            .request_multipart(method, path, form, Self::with_skills_beta(options))
            .await
    }

    /// Convert a local directory into skill upload files.
    fn collect_dir_files(root: &Path) -> Result<Vec<std::path::PathBuf>> {
        let root_metadata = std::fs::symlink_metadata(root).map_err(|e| {
            AnthropicError::file_error(format!(
                "Failed to read directory metadata {}: {}",
                root.display(),
                e
            ))
        })?;

        if root_metadata.file_type().is_symlink() {
            return Err(AnthropicError::file_error(format!(
                "Symlinks are not allowed in skill directories: {}",
                root.display()
            )));
        }

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
                let file_type = entry.file_type().map_err(|e| {
                    AnthropicError::file_error(format!(
                        "Failed to read file type for {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                if file_type.is_symlink() {
                    return Err(AnthropicError::file_error(format!(
                        "Symlinks are not allowed in skill directories: {}",
                        path.display()
                    )));
                }

                if file_type.is_dir() {
                    stack.push(path);
                } else if file_type.is_file() {
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
            validate_skill_pagination(params.limit, params.page.as_deref())?;
            if let Some(limit) = params.limit {
                query_params.push(format!("limit={}", limit));
            }
            if let Some(page) = params.page {
                query_params.push(format!("page={}", encode_query_value(&page)));
            }
            if let Some(source) = params.source {
                query_params.push(format!("source={}", encode_query_value(&source)));
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
        self.list_all_with_limits(
            SkillListParams::new().with_limit(100),
            PaginationLimits::default(),
            options,
        )
        .await
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
            validate_skill_pagination(params.limit, params.page.as_deref())?;
            if let Some(limit) = params.limit {
                query_params.push(format!("limit={}", limit));
            }
            if let Some(page) = params.page {
                query_params.push(format!("page={}", encode_query_value(&page)));
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
        self.list_all_versions_with_limits(
            skill_id,
            SkillVersionListParams::new().with_limit(100),
            PaginationLimits::default(),
            options,
        )
        .await
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
    /// Lazily traverse legacy Skills pages while preserving filters and options.
    pub fn pages(
        &self,
        mut params: SkillListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<Skill>> {
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        params.limit = params.limit.or(Some(100));
        let initial = params.page.take();
        let api = self.clone();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list(Some(params), options).await?;
                let next_cursor = legacy_skill_cursor(response.has_more, response.next_page)?;
                let item_ids = response.data.iter().map(|skill| skill.id.clone()).collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor,
                    item_ids,
                })
            }
        })
    }

    /// Collect Skills with explicit finite traversal ceilings.
    pub async fn list_all_with_limits(
        &self,
        params: SkillListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<Skill>> {
        self.pages(params, limits, options)?.collect_items().await
    }

    /// Lazily traverse versions on the historical beta path.
    pub fn version_pages(
        &self,
        skill_id: &str,
        mut params: SkillVersionListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<SkillVersion>> {
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        params.limit = params.limit.or(Some(100));
        let initial = params.page.take();
        let api = self.clone();
        let skill_id = skill_id.to_string();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let skill_id = skill_id.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list_versions(&skill_id, Some(params), options).await?;
                let next_cursor = legacy_skill_cursor(response.has_more, response.next_page)?;
                let item_ids = response
                    .data
                    .iter()
                    .map(|version| version.id.clone())
                    .collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor,
                    item_ids,
                })
            }
        })
    }

    /// Collect every legacy version, failing instead of truncating at a limit.
    pub async fn list_all_versions_with_limits(
        &self,
        skill_id: &str,
        params: SkillVersionListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<SkillVersion>> {
        self.version_pages(skill_id, params, limits, options)?
            .collect_items()
            .await
    }
}

fn validate_skill_pagination(limit: Option<u32>, page: Option<&str>) -> Result<()> {
    if limit.is_some_and(|value| !(1..=1000).contains(&value)) {
        return Err(AnthropicError::invalid_input(
            "Skill page size must be between 1 and 1000",
        ));
    }
    if page == Some("") {
        return Err(AnthropicError::invalid_input(
            "Skill page token must not be empty",
        ));
    }
    Ok(())
}

fn legacy_skill_cursor(has_more: bool, next_page: Option<String>) -> Result<Option<String>> {
    if has_more && next_page.is_none() {
        return Err(AnthropicError::invalid_input(
            "Legacy Skills response has_more without next_page",
        ));
    }
    // Token cursors themselves determine continuation, even if has_more was omitted.
    Ok(next_page)
}

fn current_skill_options(options: Option<RequestOptions>) -> Result<Option<RequestOptions>> {
    if let Some(options) = &options {
        let conflicting_header = options.headers.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("anthropic-beta")
                && value
                    .split(',')
                    .any(|beta| beta.trim() == beta_headers::SKILLS_API)
        });
        if options.enable_skills_api
            || options.beta_features.iter().any(|entry| {
                entry
                    .split(',')
                    .any(|beta| beta.trim() == beta_headers::SKILLS_API)
            })
            || conflicting_header
        {
            return Err(AnthropicError::invalid_input(
                "Current Skills cannot use skills-2025-10-02; use skills_legacy() for that schema",
            ));
        }
    }
    Ok(options)
}

fn skill_path_segment(id: &str) -> Result<String> {
    if id.is_empty() || matches!(id, "." | "..") {
        return Err(AnthropicError::invalid_input(
            "Skill and version IDs must not be empty or dot path segments",
        ));
    }
    Ok(id
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect())
}

/// Current Skills client, using object sources and version IDs without the dated beta header.
///
/// Existing [`SkillsApi`] calls retain their legacy schema. Explicit unrelated beta and
/// workspace headers are preserved; choosing the legacy Skills header here is an error.
#[derive(Clone)]
pub struct CurrentSkillsApi {
    client: Client,
}

impl CurrentSkillsApi {
    /// Construct the current-schema Skills client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List one current page; continuation is represented by `next_page`.
    pub async fn list(
        &self,
        params: Option<SkillListParams>,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillListResponse> {
        let params = params.unwrap_or_default();
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={limit}"));
        }
        if let Some(page) = params.page {
            query.push(format!("page={}", encode_query_value(&page)));
        }
        if let Some(source) = params.source {
            query.push(format!("source={}", encode_query_value(&source)));
        }
        self.client
            .request(
                HttpMethod::Get,
                &build_path_with_query("/skills", query),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// Retrieve a current skill and its latest-version ID.
    pub async fn get(
        &self,
        skill_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkill> {
        self.client
            .request(
                HttpMethod::Get,
                &format!("/skills/{}", skill_path_segment(skill_id)?),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// Create a skill with `display_name` and validated upload layout.
    pub async fn create(
        &self,
        request: CurrentSkillCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkill> {
        request.validate()?;
        let mut form = SkillsApi::build_skill_upload_form(None, request.files)?;
        if let Some(name) = request.display_name {
            form = form.text("display_name", name);
        }
        self.client
            .request_multipart(
                HttpMethod::Post,
                "/skills",
                form,
                current_skill_options(options)?,
            )
            .await
    }

    /// Create a current skill from a local directory, refusing symlinks.
    pub async fn create_from_dir(
        &self,
        dir: impl AsRef<Path>,
        display_name: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkill> {
        let files = SkillsApi::build_upload_files_from_dir(dir.as_ref()).await?;
        let mut request = CurrentSkillCreateRequest::new();
        request.files = files;
        if let Some(name) = display_name {
            request = request.display_name(name);
        }
        self.create(request, options).await
    }

    /// Delete a current skill.
    pub async fn delete(
        &self,
        skill_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SkillDeleteResponse> {
        self.client
            .request(
                HttpMethod::Delete,
                &format!("/skills/{}", skill_path_segment(skill_id)?),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// List current version IDs and metadata.
    pub async fn list_versions(
        &self,
        skill_id: &str,
        params: Option<SkillVersionListParams>,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillVersionListResponse> {
        let params = params.unwrap_or_default();
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={limit}"));
        }
        if let Some(page) = params.page {
            query.push(format!("page={}", encode_query_value(&page)));
        }
        self.client
            .request(
                HttpMethod::Get,
                &build_path_with_query(
                    &format!("/skills/{}/versions", skill_path_segment(skill_id)?),
                    query,
                ),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// Retrieve a version by ID, or the literal `latest`.
    pub async fn get_version(
        &self,
        skill_id: &str,
        version_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillVersion> {
        self.client
            .request(
                HttpMethod::Get,
                &format!(
                    "/skills/{}/versions/{}",
                    skill_path_segment(skill_id)?,
                    skill_path_segment(version_id)?
                ),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// Upload a new version; paths use version IDs rather than epoch timestamps.
    pub async fn create_version(
        &self,
        skill_id: &str,
        request: CurrentSkillVersionCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillVersion> {
        request.validate()?;
        let form = SkillsApi::build_skill_upload_form(None, request.files)?;
        self.client
            .request_multipart(
                HttpMethod::Post,
                &format!("/skills/{}/versions", skill_path_segment(skill_id)?),
                form,
                current_skill_options(options)?,
            )
            .await
    }

    /// Convenience alias: updates create a new immutable version.
    pub async fn update(
        &self,
        skill_id: &str,
        request: CurrentSkillVersionCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillVersion> {
        self.create_version(skill_id, request, options).await
    }

    /// Upload a version from a symlink-free directory.
    pub async fn create_version_from_dir(
        &self,
        skill_id: &str,
        dir: impl AsRef<Path>,
        options: Option<RequestOptions>,
    ) -> Result<CurrentSkillVersion> {
        let files = SkillsApi::build_upload_files_from_dir(dir.as_ref()).await?;
        let mut request = CurrentSkillVersionCreateRequest::new();
        request.files = files;
        self.create_version(skill_id, request, options).await
    }

    /// Delete a current version by its ID.
    pub async fn delete_version(
        &self,
        skill_id: &str,
        version_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SkillVersionDeleteResponse> {
        self.client
            .request(
                HttpMethod::Delete,
                &format!(
                    "/skills/{}/versions/{}",
                    skill_path_segment(skill_id)?,
                    skill_path_segment(version_id)?
                ),
                None,
                current_skill_options(options)?,
            )
            .await
    }

    /// Traverse current skill pages lazily, preserving source and request options.
    pub fn pages(
        &self,
        mut params: SkillListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<CurrentSkill>> {
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        current_skill_options(options.clone())?;
        params.limit = params.limit.or(Some(100));
        let initial = params.page.take();
        let api = self.clone();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list(Some(params), options).await?;
                let item_ids = response.data.iter().map(|skill| skill.id.clone()).collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor: response.next_page,
                    item_ids,
                })
            }
        })
    }

    /// Collect current skill pages with finite default ceilings.
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<CurrentSkill>> {
        self.list_all_with_limits(SkillListParams::new(), PaginationLimits::default(), options)
            .await
    }

    /// Collect current skill pages, failing at an explicit ceiling.
    pub async fn list_all_with_limits(
        &self,
        params: SkillListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<CurrentSkill>> {
        self.pages(params, limits, options)?.collect_items().await
    }

    /// Traverse current versions lazily and independently of `has_more`.
    pub fn version_pages(
        &self,
        skill_id: &str,
        mut params: SkillVersionListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<CurrentSkillVersion>> {
        validate_skill_pagination(params.limit, params.page.as_deref())?;
        skill_path_segment(skill_id)?;
        current_skill_options(options.clone())?;
        params.limit = params.limit.or(Some(100));
        let initial = params.page.take();
        let api = self.clone();
        let skill_id = skill_id.to_string();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let skill_id = skill_id.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list_versions(&skill_id, Some(params), options).await?;
                let item_ids = response
                    .data
                    .iter()
                    .map(|version| version.id.clone())
                    .collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor: response.next_page,
                    item_ids,
                })
            }
        })
    }

    /// Collect every current version subject to finite defaults.
    pub async fn list_all_versions(
        &self,
        skill_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<CurrentSkillVersion>> {
        self.list_all_versions_with_limits(
            skill_id,
            SkillVersionListParams::new(),
            PaginationLimits::default(),
            options,
        )
        .await
    }

    /// Collect current versions subject to explicit finite limits.
    pub async fn list_all_versions_with_limits(
        &self,
        skill_id: &str,
        params: SkillVersionListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<CurrentSkillVersion>> {
        self.version_pages(skill_id, params, limits, options)?
            .collect_items()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::SkillsApi;
    use tempfile::tempdir;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

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

    #[cfg(unix)]
    #[tokio::test]
    async fn test_build_upload_files_from_dir_rejects_symlinks() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("my_skill");
        std::fs::create_dir_all(&root).unwrap();

        let external_file = dir.path().join("secret.txt");
        std::fs::write(&external_file, "secret").unwrap();
        symlink(&external_file, root.join("leak.txt")).unwrap();

        let err = SkillsApi::build_upload_files_from_dir(&root)
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("Symlinks are not allowed"));
    }
}
