use super::*;

pub(super) fn current_blocks() -> Vec<Value> {
    vec![
        json!({"type":"search_result","source":"https://example.org/research","title":"Research","content":[{"type":"text","text":"Evidence","revision":3}],"citations":{"enabled":true,"policy":{"next":true}},"cache_control":{"type":"ephemeral","ttl":"1h","new_cache":[]},"score":0.75}),
        json!({"type":"container_upload","file_id":"file_123","cache_control":{"type":"ephemeral","future":"retained"},"upload_status":"ready"}),
        json!({"type":"mcp_tool_use","id":"mcp_123","name":"lookup","server_name":"research","input":{"query":"example","nested":[1,2]},"server_revision":42}),
        json!({"type":"compaction","content":"summary","encrypted_content":"opaque","signature":"signed","tool_changes":[{"type":"tool_addition","tool":{"name":"lookup","new":null}}],"cache_control":{"type":"ephemeral","future":{}},"boundary":{"from":4,"to":9}}),
    ]
}

pub(super) fn message_fixture(content: Vec<Value>) -> Value {
    json!({"id":"msg_1","type":"message","role":"assistant","model":"future-model","content":content,"stop_reason":"future_stop","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":4}})
}

pub(super) fn fixture_client(server: &MockServer) -> Client {
    Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    )
}
