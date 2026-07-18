//! Top-level agent-readable capability index for the `coracore` binary.

use std::io::{self, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

const RUNTIME_ENV: [&str; 4] = [
    "CORA_COWORK_HELPER_BIN",
    "CORA_COWORK_BASE_URL",
    "CORA_COWORK_CONVERSATION_ID",
    "CORA_COWORK_USER_ID",
];

pub(crate) fn run_capabilities() -> ExitCode {
    match print_envelope(data()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => {
            eprintln!("CAPABILITIES_STDOUT_WRITE_FAILED command=\"capabilities\": failed to write JSON output");
            ExitCode::from(1)
        }
    }
}

fn data() -> Value {
    json!({
        "schema_version": 1,
        "contract": "agent-facing-coracore-cli",
        "stability": "stable",
        "entrypoint": "coracore capabilities",
        "purpose": "Top-level index for agent-facing CoraCore CLI domains.",
        "output": {
            "stdout": "JSON envelope",
            "stderr": "single stable ..._FAILED error line when output cannot be written",
            "success_shape": {
                "success": true,
                "data": {},
                "meta": {
                    "schema_version": 1
                }
            }
        },
        "runtime_context": {
            "primary": "CORA_COWORK_CONVERSATION_ID",
            "environment": RUNTIME_ENV,
            "selectors": {
                "conversation_id": {
                    "current": "resolve from CORA_COWORK_CONVERSATION_ID"
                },
                "assistant_id": {
                    "current": "resolve via current conversation"
                },
                "user_id": {
                    "current": "resolve from CORA_COWORK_USER_ID"
                }
            }
        },
        "input": {
            "default_mode": "stdin_json",
            "business_flags": false,
            "domain_contracts": "Use each domain's capabilities command for exact stdin fields and safety metadata."
        },
        "domains": [
            {
                "name": "config",
                "mode": "read-write",
                "description": "Manage CoraCowork configuration: assistants, assistant rules, skills, MCP servers, providers, settings, agents, and scheduled tasks.",
                "contract": "agent-facing-config-cli",
                "contract_command": "config capabilities",
                "invocation": "coracore config capabilities",
                "runtime_required": ["CORA_COWORK_BASE_URL", "CORA_COWORK_CONVERSATION_ID", "CORA_COWORK_USER_ID"],
                "safety": {
                    "can_write": true,
                    "read_before_write": true,
                    "redacted_by_default": true
                }
            },
            {
                "name": "diagnose",
                "mode": "read-only",
                "description": "Diagnose a running CoraCowork installation: backend health, conversations, provider health, MCP, cron, teams, logs, and controlled GET reads.",
                "contract": "agent-facing-diagnose-cli",
                "contract_command": "diagnose capabilities",
                "invocation": "coracore diagnose capabilities",
                "runtime_required": ["CORA_COWORK_BASE_URL", "CORA_COWORK_CONVERSATION_ID", "CORA_COWORK_USER_ID"],
                "optional_runtime": ["CORA_COWORK_LOG_DIR"],
                "safety": {
                    "can_write": false,
                    "read_only": true,
                    "redacted_by_default": true,
                    "escape_hatch": "diagnose http get"
                }
            },
            {
                "name": "team",
                "mode": "team-collaboration",
                "description": "Agent-facing Team collaboration CLI fallback for agents without MCP injection.",
                "contract": "agent-facing-team-cli",
                "contract_command": "team capabilities",
                "invocation": "coracore team capabilities",
                "runtime_required": ["CORA_COWORK_BASE_URL", "CORA_COWORK_CONVERSATION_ID", "CORA_COWORK_USER_ID", "CORA_COWORK_RUNTIME_TOKEN"],
                "runtime_free_commands": ["team capabilities", "team help"],
                "safety": {
                    "can_write": true,
                    "runtime_token_required_for_context_and_call": true,
                    "does_not_accept_identity_authority_from_stdin": true
                }
            }
        ],
        "non_agent_subcommands": [
            {
                "name": "doctor",
                "description": "Human/developer self-check for agent backend availability."
            },
            {
                "name": "mcp-bridge",
                "description": "Internal stdio to TCP bridge for team MCP."
            },
            {
                "name": "mcp-team-stdio",
                "description": "Internal team MCP stdio server."
            },
            {
                "name": "prepare-managed-resources",
                "description": "Packaging helper for managed runtime resources."
            }
        ]
    })
}

fn print_envelope(data: Value) -> Result<(), ()> {
    let rendered = serde_json::to_string_pretty(&json!({
        "success": true,
        "data": data,
        "meta": {
            "schema_version": 1
        }
    }))
    .map_err(|_| ())?;
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(rendered.as_bytes())
        .and_then(|_| stdout.write_all(b"\n"))
        .map_err(|_| ())
}
