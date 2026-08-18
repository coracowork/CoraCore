use cora_cowork_db::{IAgentMetadataRepository, SqliteAgentMetadataRepository, init_database_memory};

/// Migration 038: the builtin corars agent (Cora CLI, seed id `632f31d2`)
/// carries a constructed at-turn fork capability — the same shape 036 wrote
/// for codex (turn anchors are stamped by the corars manager + engine).
#[tokio::test]
async fn corars_builtin_agent_declares_at_turn_fork_capability() {
    let db = init_database_memory().await.unwrap();
    let repo = SqliteAgentMetadataRepository::new(db.pool().clone());

    let corars = repo.get("632f31d2").await.unwrap().expect("seeded Cora CLI row");
    assert_eq!(corars.agent_type, "corars");
    assert_eq!(corars.backend, None, "corars resolves by agent_type, not backend");

    let capabilities: serde_json::Value =
        serde_json::from_str(corars.agent_capabilities.as_deref().expect("constructed capabilities")).unwrap();
    assert_eq!(
        capabilities["session_capabilities"]["fork"],
        serde_json::json!({"at_turn": true}),
        "at-turn fork: anchors are stamped on corars rows and session messages"
    );
}
