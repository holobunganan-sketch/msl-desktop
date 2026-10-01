//! OpenCode's exact endpoint contract; catalog guidance never overrides a saved route.

use crate::db::provider::ProviderConnection;

fn official_origin(url: &reqwest::Url) -> bool {
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    if url.scheme() == "https" && url.host_str() == Some("opencode.ai") && url.port().is_none() {
        return true;
    }
    // Isolated unit and native-app tests exercise the contract on local mocks.
    std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1")
        && matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        })
}

fn service(url: &reqwest::Url) -> Option<(&'static str, &'static str)> {
    if !official_origin(url) {
        return None;
    }
    match url.path().trim_end_matches('/') {
        "/zen/go" | "/zen/go/v1" => Some(("opencode_go", "/zen/go/v1")),
        "/zen" | "/zen/v1" => Some(("opencode_zen", "/zen/v1")),
        _ => None,
    }
}

fn protocol_endpoint(protocol: &str) -> Option<&'static str> {
    match protocol {
        "chat_completions" => Some("/chat/completions"),
        "responses" => Some("/responses"),
        "anthropic_messages" => Some("/messages"),
        _ => None,
    }
}

/// Normalize only documented base + protocol endpoint pairs. Custom paths remain explicit.
pub fn normalized_endpoint(base: &reqwest::Url, protocol: &str, endpoint: &str) -> Option<String> {
    let (_, versioned_base) = service(base)?;
    (protocol_endpoint(protocol)? == endpoint).then(|| format!("{versioned_base}{endpoint}"))
}

/// The effective URL, including the selected protocol, controls special wire behavior.
pub fn is_request(url: &reqwest::Url, protocol: &str) -> bool {
    official_origin(url)
        && protocol_endpoint(protocol).is_some_and(|endpoint| {
            ["/zen/go/v1", "/zen/v1"]
                .iter()
                .any(|base| url.path() == format!("{base}{endpoint}"))
        })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoutingOption {
    pub protocol: &'static str,
    pub endpoint_path: &'static str,
    pub recommended: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoutingGuidance {
    pub recommended_protocol: &'static str,
    pub options: Vec<RoutingOption>,
    pub note: &'static str,
    pub source_url: &'static str,
}

pub fn routing_guidance(
    connection: &ProviderConnection,
    model_id: &str,
) -> Option<RoutingGuidance> {
    let base = reqwest::Url::parse(&connection.base_url).ok()?;
    let (kind, _) = service(&base)?;
    let recommended = crate::ai::catalog::known_model(kind, model_id)?;
    protocol_endpoint(recommended.protocol)?;
    let mut options = vec![RoutingOption {
        protocol: recommended.protocol,
        endpoint_path: recommended.endpoint_path,
        recommended: true,
    }];
    let mut note = "按当前官方模型目录推荐协议；手动保存的协议和端点会继续生效。";
    let mut source_url = if kind == "opencode_go" {
        "https://opencode.ai/docs/go/"
    } else {
        "https://opencode.ai/docs/zen/"
    };
    if kind == "opencode_go" {
        // Pinned Reasonix compatibility routes supplement current official defaults.
        // Exact IDs keep later model releases from inheriting unverified routes.
        let alternates: &[&str] = match model_id {
            "deepseek-v4-flash" => {
                note = "推荐 Chat Completions；Reasonix 另列 Messages 和 Responses 兼容入口，可手动选择。";
                &["anthropic_messages", "responses"]
            }
            "qwen3.6-plus" | "qwen3.7-plus" | "qwen3.7-max" | "qwen3.8-max" | "minimax-m2.7" => {
                note = "推荐当前官方 Messages 入口；Reasonix 锁定的 Pi 目录另列 Chat Completions 兼容入口，可手动选择。";
                &["chat_completions"]
            }
            _ => &[],
        };
        if model_id == "deepseek-v4-flash" {
            source_url = "https://github.com/indielab/Reasonix/blob/6845b6de3c5e079737a3b010ebcdd87f6cd0228c/internal/config/provider_presets_opencode_go.go";
        } else if !alternates.is_empty() {
            source_url = "https://github.com/sky-valley/pi/blob/886ece5a0eaf8f1cd5a25f0c8138f9c8a65ad6dd/ai/models_catalog.json";
        }
        options.extend(alternates.iter().map(|protocol| RoutingOption {
            protocol,
            endpoint_path: protocol_endpoint(protocol).expect("listed compatibility protocol"),
            recommended: false,
        }));
    }
    Some(RoutingGuidance {
        recommended_protocol: recommended.protocol,
        options,
        note,
        source_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{provider::ProviderCatalogRepo, Database};

    fn connection(base: &str) -> ProviderConnection {
        let db = Database::open_in_memory().unwrap();
        ProviderCatalogRepo::new(db.conn())
            .insert_connection(
                "Synthetic OpenCode",
                "custom",
                base,
                "",
                "opencode_go",
                "bearer",
                None,
                true,
            )
            .unwrap()
    }

    #[test]
    fn opencode_contract_guidance_keeps_current_defaults_and_only_verified_alternates() {
        let go = connection("https://opencode.ai/zen/go/v1");
        for (id, recommended, choices) in [
            (
                "deepseek-v4-flash",
                "chat_completions",
                vec!["chat_completions", "anthropic_messages", "responses"],
            ),
            (
                "deepseek-v4-pro",
                "chat_completions",
                vec!["chat_completions"],
            ),
            (
                "qwen3.6-plus",
                "anthropic_messages",
                vec!["anthropic_messages", "chat_completions"],
            ),
            (
                "qwen3.7-plus",
                "anthropic_messages",
                vec!["anthropic_messages", "chat_completions"],
            ),
            (
                "qwen3.7-max",
                "anthropic_messages",
                vec!["anthropic_messages", "chat_completions"],
            ),
            (
                "qwen3.8-max",
                "anthropic_messages",
                vec!["anthropic_messages", "chat_completions"],
            ),
            (
                "minimax-m2.7",
                "anthropic_messages",
                vec!["anthropic_messages", "chat_completions"],
            ),
            (
                "minimax-m3",
                "anthropic_messages",
                vec!["anthropic_messages"],
            ),
            (
                "qwen3.8-flash",
                "anthropic_messages",
                vec!["anthropic_messages"],
            ),
            ("grok-4.6", "responses", vec!["responses"]),
            (
                "deepseek-v4-flash-vision-exp",
                "chat_completions",
                vec!["chat_completions"],
            ),
        ] {
            let guidance = routing_guidance(&go, id).expect(id);
            assert_eq!(guidance.recommended_protocol, recommended, "{id}");
            assert_eq!(
                guidance
                    .options
                    .iter()
                    .map(|o| o.protocol)
                    .collect::<Vec<_>>(),
                choices,
                "{id}"
            );
            assert_eq!(guidance.options.iter().filter(|o| o.recommended).count(), 1);
            for option in &guidance.options {
                assert_eq!(
                    option.endpoint_path,
                    match option.protocol {
                        "chat_completions" => "/chat/completions",
                        "anthropic_messages" => "/messages",
                        "responses" => "/responses",
                        other => panic!("unrecognized protocol {other}"),
                    }
                );
            }
        }
        let zen = connection("https://opencode.ai/zen/v1");
        let minimax = routing_guidance(&zen, "minimax-m3").unwrap();
        assert_eq!(minimax.recommended_protocol, "chat_completions");
        assert_eq!(minimax.options.len(), 1);
        for unknown in [
            "deepseek-v4-future",
            "deepseek-v4-flash-new",
            "qwen3.9-max",
            "minimax-m4",
        ] {
            assert!(routing_guidance(&go, unknown).is_none(), "{unknown}");
        }
        assert!(routing_guidance(&zen, "gemini-3.8-flash").is_none());
    }

    #[test]
    fn opencode_contract_guidance_uses_exact_official_origin_and_base_path() {
        for base in [
            "https://opencode.ai/zen/go",
            "https://opencode.ai/zen/go/v1/",
            "https://opencode.ai/zen",
            "https://opencode.ai/zen/v1",
            "http://127.0.0.1:32123/zen/go/v1",
            "http://[::1]:32123/zen/v1",
        ] {
            assert!(
                routing_guidance(&connection(base), "deepseek-v4-flash").is_some(),
                "{base}"
            );
        }
        for base in [
            "https://proxy.example/zen/go/v1",
            "https://opencode.ai.example/zen/go/v1",
            "http://opencode.ai/zen/go/v1",
            "https://opencode.ai:444/zen/go/v1",
            "https://user@opencode.ai/zen/go/v1",
            "https://opencode.ai/zen/go/v1?key=x",
            "https://opencode.ai/zen/go/v1#part",
            "https://opencode.ai/custom/v1",
            "https://opencode.ai/zen/go/v1/other",
            "https://opencode.ai/zen/go/v1/messages",
            "http://127.0.0.1:32123/proxy/v1",
        ] {
            assert!(
                routing_guidance(&connection(base), "deepseek-v4-flash").is_none(),
                "{base}"
            );
        }
    }

    #[test]
    fn opencode_contract_request_checks_effective_url_and_protocol_together() {
        for (url, protocol, expected) in [
            (
                "https://opencode.ai/zen/go/v1/messages",
                "anthropic_messages",
                true,
            ),
            ("https://opencode.ai/zen/v1/responses", "responses", true),
            (
                "https://opencode.ai/zen/go/v1/chat/completions",
                "chat_completions",
                true,
            ),
            (
                "https://proxy.example/zen/go/v1/messages",
                "anthropic_messages",
                false,
            ),
            (
                "https://opencode.ai.example/zen/go/v1/messages",
                "anthropic_messages",
                false,
            ),
            (
                "https://opencode.ai/zen/go/v1/messages?key=x",
                "anthropic_messages",
                false,
            ),
            (
                "https://opencode.ai/custom/v1/messages",
                "anthropic_messages",
                false,
            ),
            (
                "https://opencode.ai/zen/go/v1/responses",
                "anthropic_messages",
                false,
            ),
            (
                "https://opencode.ai/zen/go/messages",
                "anthropic_messages",
                false,
            ),
        ] {
            assert_eq!(
                is_request(&reqwest::Url::parse(url).unwrap(), protocol),
                expected,
                "{url}/{protocol}"
            );
        }
    }
}
