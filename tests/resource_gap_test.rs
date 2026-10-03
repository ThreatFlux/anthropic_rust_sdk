//! Current resource schemas, bounded traversal, and genuinely incremental batch results.
//! Fixtures follow official Python SDK revision 18f25547f20cf5f01da69ac611e700e3bc9ebf21.

use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use threatflux_anthropic_sdk::{
    api::message_batches::BatchResultsStreamOptions,
    models::{
        file::{File, FileListParams, FileUploadRequest},
        skill::{
            CurrentSkillCreateRequest, CurrentSkillVersionCreateRequest, SkillFileUpload,
            SkillListParams, SkillVersionListParams,
        },
    },
    types::{Pagination, PaginationLimits, RequestOptions},
    Client, Config,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

#[path = "resource_gap_test/files.rs"]
mod files;
#[path = "resource_gap_test/jsonl.rs"]
mod jsonl;
#[path = "resource_gap_test/pagination.rs"]
mod pagination;
#[path = "resource_gap_test/skills.rs"]
mod skills;
#[path = "resource_gap_test/support.rs"]
mod support;

use support::*;
