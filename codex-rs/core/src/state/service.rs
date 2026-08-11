use std::collections::HashMap;
use std::sync::Arc;

use crate::SkillsManager;
use crate::agent::AgentControl;
use crate::attestation::AttestationProvider;
use crate::client::ModelClient;
use crate::config::NetworkProxyAuditMetadata;
use crate::config::StartedNetworkProxy;
use crate::exec_policy::ExecPolicyManager;
use crate::guardian::GuardianRejection;
use crate::guardian::GuardianRejectionCircuitBreaker;
use crate::mcp::McpManager;
use crate::tools::code_mode::CodeModeService;
use crate::tools::network_approval::NetworkApprovalService;
use crate::tools::sandboxing::ApprovalStore;
use crate::unified_exec::UnifiedExecProcessManager;
use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use codex_analytics::AnalyticsEventsClient;
use codex_core_plugins::PluginsManager;
use codex_exec_server::EnvironmentManager;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistry;
use codex_hooks::Hooks;
use codex_login::AuthManager;
use codex_mcp::McpConnectionManager;
use codex_models_manager::manager::SharedModelsManager;
use codex_otel::SessionTelemetry;
use codex_rollout::state_db::StateDbHandle;
use codex_rollout_trace::ThreadTraceContext;
use codex_thread_store::LiveThread;
use codex_thread_store::ThreadStore;
use std::path::PathBuf;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub(crate) struct SessionServices {
    pub(crate) mcp_connection_manager: Arc<RwLock<McpConnectionManager>>,
    pub(crate) mcp_startup_cancellation_token: Mutex<CancellationToken>,
    pub(crate) unified_exec_manager: UnifiedExecProcessManager,
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(crate) shell_zsh_path: Option<PathBuf>,
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(crate) main_execve_wrapper_exe: Option<PathBuf>,
    pub(crate) analytics_events_client: AnalyticsEventsClient,
    pub(crate) hooks: ArcSwap<Hooks>,
    pub(crate) rollout_thread_trace: ThreadTraceContext,
    pub(crate) user_shell: Arc<crate::shell::Shell>,
    pub(crate) shell_snapshot_tx: watch::Sender<Option<Arc<crate::shell_snapshot::ShellSnapshot>>>,
    pub(crate) show_raw_agent_reasoning: bool,
    pub(crate) exec_policy: Arc<ExecPolicyManager>,
    pub(crate) auth_manager: Arc<AuthManager>,
    /// Provider-bound model catalog manager, swapped on a mid-session provider
    /// switch (see `Session::update_settings` and `set_models_manager`) so
    /// per-turn metadata resolution (`get_model_info`) follows the active
    /// provider — e.g. an OpenRouter manager resolves free-typed slugs via the
    /// provider-native catalog hook that a manager bound to a different
    /// provider would not. Guarded by a `RwLock` because the swap happens
    /// through `&self`; readers clone the `Arc` out (`models_manager()`) so no
    /// guard is held across `.await`.
    pub(crate) models_manager_cell: std::sync::RwLock<SharedModelsManager>,
    pub(crate) session_telemetry: SessionTelemetry,
    pub(crate) tool_approvals: Mutex<ApprovalStore>,
    pub(crate) guardian_rejections: Mutex<HashMap<String, GuardianRejection>>,
    pub(crate) guardian_rejection_circuit_breaker: Mutex<GuardianRejectionCircuitBreaker>,
    pub(crate) runtime_handle: Handle,
    pub(crate) skills_manager: Arc<SkillsManager>,
    pub(crate) plugins_manager: Arc<PluginsManager>,
    pub(crate) mcp_manager: Arc<McpManager>,
    pub(crate) extensions: Arc<ExtensionRegistry<crate::config::Config>>,
    pub(crate) session_extension_data: ExtensionData,
    pub(crate) thread_extension_data: ExtensionData,
    pub(crate) agent_control: AgentControl,
    pub(crate) network_proxy: ArcSwapOption<StartedNetworkProxy>,
    pub(crate) network_proxy_audit_metadata: NetworkProxyAuditMetadata,
    pub(crate) managed_network_requirements_configured: bool,
    pub(crate) network_approval: Arc<NetworkApprovalService>,
    pub(crate) state_db: Option<StateDbHandle>,
    pub(crate) live_thread: Option<LiveThread>,
    pub(crate) thread_store: Arc<dyn ThreadStore>,
    pub(crate) attestation_provider: Option<Arc<dyn AttestationProvider>>,
    /// Session-scoped model client shared across turns.
    pub(crate) model_client: ModelClient,
    pub(crate) code_mode_service: CodeModeService,
    /// Shared process-level environment registry. Sessions carry an `Arc` handle so they can pass
    /// the same manager through child-thread spawn paths without reconstructing it.
    pub(crate) environment_manager: Arc<EnvironmentManager>,
}

impl SessionServices {
    /// Snapshot the active provider-bound models manager as an owned handle.
    ///
    /// The lock is released before the clone returns, so callers may freely
    /// `.await` on the manager (e.g. `get_model_info`, which fetches the
    /// OpenRouter catalog for a free-typed slug) without holding the guard.
    pub(crate) fn models_manager(&self) -> SharedModelsManager {
        self.models_manager_cell
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Rebind the models manager to a new provider. Used by
    /// `Session::update_settings` on a mid-session provider switch.
    pub(crate) fn set_models_manager(&self, manager: SharedModelsManager) {
        *self
            .models_manager_cell
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = manager;
    }
}
