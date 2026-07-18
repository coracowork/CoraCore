use cora_agent::error::AgentError as CorarsAgentError;
use cora_cowork_api_types::{
    AgentErrorCode, AgentErrorOwnership, AgentErrorResolution, AgentErrorResolutionKind, AgentErrorResolutionTarget,
};
use cora_providers::ProviderError;

use crate::protocol::send_error::AgentSendError;

pub(super) fn corars_engine_error_to_send_error(error: &CorarsAgentError) -> AgentSendError {
    let detail = format!("Corars agent error: {error}");
    match error {
        CorarsAgentError::Provider(provider_error) => corars_provider_error_to_send_error(provider_error, detail),
        CorarsAgentError::ToolCallMalformed { .. } => provider_send_error(
            "The model provider repeatedly returned malformed tool calls",
            AgentErrorCode::UserLlmProviderInvalidRequest,
            detail,
            false,
            AgentErrorResolutionKind::ChangeModel,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        CorarsAgentError::ToolCallFailures { .. } => tool_call_failure_send_error(detail),
        CorarsAgentError::ContextTooLong { .. } => provider_send_error(
            "The request is too large for the configured model context window",
            AgentErrorCode::UserLlmProviderContextTooLarge,
            detail,
            false,
            AgentErrorResolutionKind::ReduceContext,
            None,
        ),
        CorarsAgentError::ApiError(_) => unknown_upstream_send_error(detail),
        CorarsAgentError::UserAborted => unknown_upstream_send_error(detail),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CorarsRuntimeErrorSummary {
    pub(super) kind: &'static str,
    pub(super) provider_error_class: Option<&'static str>,
    pub(super) http_status: Option<u16>,
    pub(super) failure_count: Option<usize>,
    pub(super) failure_limit: Option<usize>,
}

impl CorarsRuntimeErrorSummary {
    fn new(kind: &'static str, provider_error_class: Option<&'static str>) -> Self {
        Self {
            kind,
            provider_error_class,
            http_status: None,
            failure_count: None,
            failure_limit: None,
        }
    }
}

pub(super) fn corars_runtime_error_summary(error: &CorarsAgentError) -> CorarsRuntimeErrorSummary {
    match error {
        CorarsAgentError::Provider(ProviderError::Api { status, .. }) => CorarsRuntimeErrorSummary {
            http_status: Some(*status),
            ..CorarsRuntimeErrorSummary::new("provider", Some("http_status"))
        },
        CorarsAgentError::Provider(ProviderError::Connection(_) | ProviderError::Http(_)) => {
            CorarsRuntimeErrorSummary::new("provider", Some("network"))
        }
        CorarsAgentError::Provider(ProviderError::RateLimited { .. }) => CorarsRuntimeErrorSummary {
            http_status: Some(429),
            ..CorarsRuntimeErrorSummary::new("provider", Some("rate_limited"))
        },
        CorarsAgentError::Provider(ProviderError::PromptTooLong(_)) => {
            CorarsRuntimeErrorSummary::new("provider", Some("context_too_large"))
        }
        CorarsAgentError::Provider(ProviderError::Parse(_)) => {
            CorarsRuntimeErrorSummary::new("provider", Some("parse"))
        }
        CorarsAgentError::ToolCallFailures { count, limit } => CorarsRuntimeErrorSummary {
            kind: "tool_call_failures",
            provider_error_class: None,
            http_status: None,
            failure_count: Some(*count),
            failure_limit: Some(*limit),
        },
        CorarsAgentError::ToolCallMalformed { count, limit } => CorarsRuntimeErrorSummary {
            kind: "tool_call_malformed",
            provider_error_class: None,
            http_status: None,
            failure_count: Some(*count),
            failure_limit: Some(*limit),
        },
        CorarsAgentError::ContextTooLong { .. } => {
            CorarsRuntimeErrorSummary::new("context_too_large", Some("context_too_large"))
        }
        CorarsAgentError::ApiError(_) => CorarsRuntimeErrorSummary::new("api_error", None),
        CorarsAgentError::UserAborted => CorarsRuntimeErrorSummary::new("user_aborted", None),
    }
}

fn corars_provider_error_to_send_error(error: &ProviderError, detail: String) -> AgentSendError {
    match error {
        ProviderError::Api { status, .. } => corars_provider_status_to_send_error(*status, detail),
        ProviderError::RateLimited { body, .. } => provider_send_error(
            "The model provider rate limited the request",
            AgentErrorCode::UserLlmProviderRateLimited,
            append_provider_body(detail, body.as_deref()),
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
        ProviderError::PromptTooLong(_) => provider_send_error(
            "The request is too large for the configured model context window",
            AgentErrorCode::UserLlmProviderContextTooLarge,
            detail,
            false,
            AgentErrorResolutionKind::ReduceContext,
            None,
        ),
        ProviderError::Connection(_) | ProviderError::Http(_) => provider_send_error(
            "The model provider could not be reached",
            AgentErrorCode::UserLlmProviderNetworkError,
            detail,
            true,
            AgentErrorResolutionKind::CheckProviderBaseUrl,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        ProviderError::Parse(_) => provider_send_error(
            "The model provider returned a server error",
            AgentErrorCode::UserLlmProviderGatewayError,
            detail,
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
    }
}

fn corars_provider_status_to_send_error(status: u16, detail: String) -> AgentSendError {
    match status {
        400 => provider_send_error(
            "The model provider rejected the request",
            AgentErrorCode::UserLlmProviderInvalidRequest,
            detail,
            false,
            AgentErrorResolutionKind::SendFeedback,
            Some(AgentErrorResolutionTarget::Feedback),
        ),
        401 => provider_send_error(
            "The model provider rejected the request",
            AgentErrorCode::UserLlmProviderAuthFailed,
            detail,
            false,
            AgentErrorResolutionKind::CheckProviderCredentials,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        402 => provider_send_error(
            "The model provider account requires billing attention",
            AgentErrorCode::UserLlmProviderBillingRequired,
            detail,
            false,
            AgentErrorResolutionKind::CheckProviderBilling,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        403 => provider_send_error(
            "The model provider denied access to the request",
            AgentErrorCode::UserLlmProviderPermissionDenied,
            detail,
            false,
            AgentErrorResolutionKind::CheckProviderCredentials,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        404 => provider_send_error(
            "The model provider endpoint was not found",
            AgentErrorCode::UserLlmProviderEndpointNotFound,
            detail,
            false,
            AgentErrorResolutionKind::CheckProviderBaseUrl,
            Some(AgentErrorResolutionTarget::ProviderSettings),
        ),
        408 | 504 => provider_send_error(
            "The model provider did not respond in time",
            AgentErrorCode::UserLlmProviderTimeout,
            detail,
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
        429 => provider_send_error(
            "The model provider rate limited the request",
            AgentErrorCode::UserLlmProviderRateLimited,
            detail,
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
        500..=599 => provider_send_error(
            "The model provider returned a server error",
            AgentErrorCode::UserLlmProviderGatewayError,
            detail,
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
        _ => provider_send_error(
            "The model provider returned an error",
            AgentErrorCode::UserLlmProviderGatewayError,
            detail,
            true,
            AgentErrorResolutionKind::Retry,
            None,
        ),
    }
}

fn provider_send_error(
    message: &'static str,
    code: AgentErrorCode,
    detail: String,
    retryable: bool,
    resolution_kind: AgentErrorResolutionKind,
    resolution_target: Option<AgentErrorResolutionTarget>,
) -> AgentSendError {
    AgentSendError::new(
        message,
        code,
        AgentErrorOwnership::UserLlmProvider,
        Some(detail),
        retryable,
        false,
        Some(AgentErrorResolution::new(resolution_kind, resolution_target)),
    )
}

fn unknown_upstream_send_error(detail: String) -> AgentSendError {
    AgentSendError::new(
        "The upstream Agent failed while handling the request",
        AgentErrorCode::UnknownUpstreamError,
        AgentErrorOwnership::UnknownUpstream,
        Some(detail),
        true,
        true,
        Some(AgentErrorResolution::new(
            AgentErrorResolutionKind::SendFeedback,
            Some(AgentErrorResolutionTarget::Feedback),
        )),
    )
}

/// Append the raw upstream response body (if any) to the detail string so
/// the UI's technical-details drawer surfaces provider-specific hints such
/// as `insufficient_quota`, `payment_required`, or per-endpoint rate-limit
/// notes. The body is passed through the existing `sanitize_error_detail`
/// pipeline downstream (redaction + truncation), so no extra scrubbing is
/// needed here.
fn append_provider_body(detail: String, body: Option<&str>) -> String {
    match body.map(str::trim).filter(|b| !b.is_empty()) {
        Some(body) => format!("{detail}\nProvider response: {body}"),
        None => detail,
    }
}

fn tool_call_failure_send_error(detail: String) -> AgentSendError {
    AgentSendError::new(
        "The upstream Agent repeatedly failed while executing tool calls",
        AgentErrorCode::UnknownUpstreamError,
        AgentErrorOwnership::UnknownUpstream,
        Some(detail),
        true,
        true,
        Some(AgentErrorResolution::new(AgentErrorResolutionKind::Retry, None)),
    )
}

#[cfg(test)]
#[path = "error_test.rs"]
mod error_test;
