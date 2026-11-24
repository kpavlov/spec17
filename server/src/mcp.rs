//! MCP over stdio or streamable HTTP. Both share the tools and the MCP Apps dashboard resource.
use crate::{AppState, frontend, ops, skill};
use rmcp::{
    ErrorData, RoleServer, ServerHandler, ServiceExt,
    model::*,
    service::{RequestContext, SubscriptionContext},
    transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager},
};
use serde_json::{Value, json};

/// How often a subscription looks for a change.
const POLL: std::time::Duration = std::time::Duration::from_secs(1);
const UI_URI: &str = "ui://spec17/dashboard";
const SKILL_URI: &str = "skill://spec17/SKILL.md";
use ops::{JSON_MIME, SCHEMA_MIME, TEXT_MIME};
const UI_MIME: &str = "text/html;profile=mcp-app";

#[derive(Clone)]
struct Mcp(AppState);

// (API-25) The skill is embedded, so its entry is parsed and hashed once.
fn skill_entry() -> Result<Skill, ErrorData> {
    static ENTRY: std::sync::LazyLock<Result<Skill, String>> = std::sync::LazyLock::new(|| {
        let frontmatter = skill::SKILL.strip_prefix("---\n").and_then(|s| s.find("\n---\n").map(|end| &s[..=end]))
            .ok_or("invalid skill frontmatter")?;
        let frontmatter = yaml_serde::from_str(frontmatter).map_err(|e| format!("invalid skill frontmatter: {e}"))?;
        Ok(Skill::new(SKILL_URI, frontmatter, SkillResources::Files(vec![SkillResource::from_bytes(SKILL_URI, skill::SKILL.as_bytes())])))
    });
    ENTRY.clone().map_err(|e| ErrorData::internal_error(e, None))
}

impl ServerHandler for Mcp {
    fn get_info(&self) -> ServerConfig {
        // (API-24)
        ServerConfig::new(ServerCapabilities::builder().enable_tools().enable_resources().enable_resources_subscribe().enable_skills().build())
            .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).with_website_url(env!("CARGO_PKG_HOMEPAGE")))
            .with_instructions(format!("Deterministic spec-driven-development evidence for one repo. {}", ops::NOTE))
    }

    async fn list_skills(&self, request: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListSkillsResult, ErrorData> {
        if request.is_some_and(|params| params.cursor.is_some()) {
            return Err(ErrorData::invalid_params("invalid skill cursor", None));
        }
        Ok(ListSkillsResult::with_all_items(vec![skill_entry()?]))
    }

    async fn get_skill(&self, request: GetSkillRequestParams, _: RequestContext<RoleServer>) -> Result<GetSkillResult, ErrorData> {
        if request.uri != SKILL_URI {
            return Err(ErrorData::invalid_params("unknown skill", None));
        }
        Ok(GetSkillResult::new(skill_entry()?))
    }

    async fn list_tools(&self, _: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListToolsResult, ErrorData> {
        let tools = ops::tools()
            .iter()
            .map(|t| {
                let schema = t.schema.as_object().cloned().unwrap_or_default();
                // (API-18)
                let mut tool = Tool::new(t.name, t.description, schema).with_annotations(ToolAnnotations::new().read_only(t.read_only()));
                if let Some(Value::Object(out)) = ops::output_schema(t.name) {
                    tool = tool.with_raw_output_schema(std::sync::Arc::new(out));
                }
                if t.ui {
                    let mut meta = MetaObject::new();
                    meta.insert("ui".into(), json!({"resourceUri": UI_URI}));
                    tool.with_meta(meta)
                } else {
                    tool
                }
            })
            .collect();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(&self, request: CallToolRequestParams, _: RequestContext<RoleServer>) -> Result<CallToolResponse, ErrorData> {
        let args = Value::Object(request.arguments.unwrap_or_default());
        let (root, dash, name, port) = (self.0.root.clone(), self.0.dashboard.clone(), request.name.to_string(), self.0.mcp_port);
        let r = tokio::task::spawn_blocking(move || ops::call(root.as_deref(), &name, &args, &dash, port)).await.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(match r {
            Ok((text, data)) => {
                let mut res = CallToolResult::success(vec![ContentBlock::text(text)]);
                res.structured_content = Some(data);
                res
            }
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
        .into())
    }

    async fn list_resources(&self, _: Option<PaginatedRequestParams>, _: RequestContext<RoleServer>) -> Result<ListResourcesResult, ErrorData> {
        // a JSON resource says which schema types it: in its description and in `_meta.schema`
        let json = |uri: String, name: &str, about: &str| {
            let mut r = Resource::new(&uri, name).with_mime_type(JSON_MIME);
            match ops::schema_uri(&uri) {
                Some(schema) => {
                    let mut meta = MetaObject::new();
                    meta.insert("schema".into(), json!(schema));
                    r = r.with_description(format!("{about} Typed by the JSON Schema at {schema}.")).with_meta(meta);
                }
                None => r = r.with_description(about),
            }
            r
        };
        let mut all = vec![Resource::new(UI_URI, "spec17 dashboard").with_description("Spec-driven status dashboard (MCP Apps)").with_mime_type(UI_MIME)];
        let skill = skill_entry()?;
        all.push(Resource::new(SKILL_URI, skill.frontmatter.name).with_description(skill.frontmatter.description).with_mime_type("text/markdown"));
        for tool in ops::VIEW_TOOLS {
            all.push(Resource::new(format!("spec17://{tool}"), format!("spec17 {tool}")).with_description(ops::tool_description(tool)).with_mime_type(TEXT_MIME));
        }
        for tool in ops::VIEW_TOOLS {
            all.push(json(format!("spec17://{tool}.json"), &format!("spec17 {tool} (JSON)"), ops::tool_description(tool)));
        }
        all.extend(ops::RESOURCES.iter().map(|(uri, name, about, _)| json(uri.to_string(), name, about)));
        all.extend(ops::SCHEMAS.iter().map(|n| {
            let uri = format!("{}{n}", ops::SCHEMA_PREFIX);
            Resource::new(uri, format!("spec17 {n} schema")).with_description(format!("JSON Schema of the `{n}` result: the JSON resource spec17://{n}(.json) and the structuredContent of its tool.")).with_mime_type(SCHEMA_MIME)
        }));
        Ok(ListResourcesResult::with_all_items(all))
    }

    async fn read_resource(&self, request: ReadResourceRequestParams, _: RequestContext<RoleServer>) -> Result<ReadResourceResponse, ErrorData> {
        if request.uri == SKILL_URI {
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(skill::SKILL, SKILL_URI).with_mime_type("text/markdown")]).into());
        }
        if request.uri.starts_with("skill://") {
            return Err(ErrorData::invalid_params("unknown skill resource", None));
        }
        if request.uri != UI_URI {
            let (root, uri) = (self.0.root.clone(), request.uri.to_string());
            let text = tokio::task::spawn_blocking(move || ops::read_resource(root.as_deref(), &uri)).await.map_err(|e| ErrorData::internal_error(e.to_string(), None))?.map_err(|e| ErrorData::internal_error(e, None))?;
            return match text {
                Some((mime, t)) => Ok(ReadResourceResult::new(vec![ResourceContents::text(t, request.uri.as_str()).with_mime_type(mime)]).into()),
                None => Err(ErrorData::resource_not_found(format!("unknown resource {}", request.uri), None)),
            };
        }
        let html = frontend::index_html().unwrap_or_else(|| {
            if self.0.dashboard.is_empty() {
                "<!doctype html><meta charset=utf-8><p>Dashboard not built.</p>".into()
            } else {
                format!("<!doctype html><meta charset=utf-8><p>Dashboard not built. Open <a href=\"{0}\">{0}</a></p>", self.0.dashboard)
            }
        });
        Ok(ReadResourceResult::new(vec![ResourceContents::text(html, UI_URI).with_mime_type(UI_MIME)]).into())
    }

    // (API-08)
    /// Only the resources this server has can be subscribed to; list-changed is never offered (the lists are static).
    fn accepted_subscription_filter(&self, requested: &SubscriptionFilter) -> Option<SubscriptionFilter> {
        let mut f = SubscriptionFilter::new();
        f.resource_subscriptions = requested.resource_subscriptions.as_ref().map(|v| v.iter().filter(|u| ops::resource_uris().any(|k| k == **u)).cloned().collect());
        Some(f)
    }

    // (API-08)
    /// Polls `ops::stamp` and sends `notifications/resources/updated` for every accepted URI when the repo changed.
    /// Ends when the client cancels or the server shuts down (a graceful `subscriptions/listen` result).
    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        let stamp = || {
            let root = self.0.root.clone();
            async move { tokio::task::spawn_blocking(move || ops::stamp(root.as_deref())).await.unwrap_or(0) }
        };
        let uris = context.accepted().resource_subscriptions.clone().unwrap_or_default();
        if uris.is_empty() {
            return Ok(()); // nothing to tell: no point scanning the repo
        }
        let mut last = stamp().await;
        let mut tick = tokio::time::interval(POLL);
        loop {
            tokio::select! {
                _ = context.cancelled() => return Ok(()),
                _ = self.0.shutdown.cancelled() => return Ok(()),
                _ = tick.tick() => {
                    let now = stamp().await;
                    if now == last {
                        continue;
                    }
                    last = now;
                    for u in &uris {
                        if context.sink().notify_resource_updated(u.as_str()).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
}

// (API-22)
pub async fn serve_stdio(state: AppState) -> Result<(), String> {
    use rmcp::service::{QuitReason, ServerInitializeError};

    let stop = state.shutdown.child_token();
    let service = match Mcp(state).serve_with_ct(rmcp::transport::stdio_pipes(), stop).await {
        Ok(service) => service,
        Err(ServerInitializeError::Cancelled | ServerInitializeError::ConnectionClosed(_)) => return Ok(()),
        Err(e) => return Err(format!("stdio MCP: {e}")),
    };
    match service.waiting().await {
        Ok(QuitReason::JoinError(e)) | Err(e) => Err(format!("stdio MCP: {e}")),
        Ok(_) => Ok(()),
    }
}

pub fn service(state: AppState, hosts: &[String]) -> StreamableHttpService<impl ServerHandler, LocalSessionManager> {
    let origins: Vec<String> = hosts.iter().map(|h| format!("http://{h}")).collect();
    // no rmcp cancellation token on purpose: it would drop open streams before `listen` can send its graceful result
    let config = StreamableHttpServerConfig::default().with_allowed_hosts(hosts.to_vec()).with_allowed_origins(origins).with_legacy_session_mode(false).with_json_response(true);
    StreamableHttpService::new(move || Ok(Mcp(state.clone())), Default::default(), config)
}
