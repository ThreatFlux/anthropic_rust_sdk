//! Shared current-resource fixtures.

use super::*;

pub(super) fn client(server: &MockServer) -> Client {
    Client::new(
        Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    )
}

pub(super) fn file(id: &str) -> Value {
    json!({"id":id,"type":"file","created_at":"2026-10-03T00:00:00Z","filename":"document.txt","mime_type":"text/plain","size_bytes":12,"downloadable":true,"expires_at":"2026-10-04T00:00:00Z","future_metadata":{"nested":7}})
}

pub(super) fn skill(id: &str) -> Value {
    json!({"id":id,"type":"skill","created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","display_name":"My Skill","latest_version_id":"skv_current","source":{"type":"custom","future_source":true},"future_skill":[1,2]})
}

pub(super) fn version(id: &str) -> Value {
    json!({"id":id,"type":"skill_version","created_at":"2026-10-03T00:00:00Z","description":"Use this skill","name":"my-skill","skill_id":"skl_current","future_version":3})
}

pub(super) fn upload_file() -> SkillFileUpload {
    SkillFileUpload::new(
        "my-skill/SKILL.md",
        b"---\nname: my-skill\n---\n".to_vec(),
        "text/markdown",
    )
}
