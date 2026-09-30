use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::time::Duration;

pub(crate) const DEFAULT_BASE: &str = "https://ilinkai.weixin.qq.com";
const CHANNEL_VERSION: &str = "2.4.9";

pub(crate) fn validate_base(input: &str) -> Result<reqwest::Url, String> {
    let invalid = || "微信服务地址未通过校验".to_string();
    let url = reqwest::Url::parse(input).map_err(|_| invalid())?;
    let host = url.host_str().ok_or_else(invalid)?;
    let official =
        url.scheme() == "https" && host.ends_with(".weixin.qq.com") && url.port().is_none();
    #[cfg(test)]
    let official = official || (url.scheme() == "http" && host == "127.0.0.1");
    if !official
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
    {
        return Err(invalid());
    }
    Ok(url)
}
pub(crate) fn qr_image(content: &str) -> Result<String, String> {
    if content.is_empty() || content.len() > 4096 {
        return Err("二维码内容无效，请重新获取".into());
    }
    let code = qrcode::QrCode::new(content.as_bytes()).map_err(|_| "二维码内容无效，请重新获取")?;
    let width = code.width();
    let mut path = String::new();
    for y in 0..width {
        for x in 0..width {
            if code[(x, y)] == qrcode::Color::Dark {
                path.push_str(&format!("M{} {}h1v1h-1z", x + 4, y + 4));
            }
        }
    }
    // Only numeric coordinates derived by the encoder enter this generated SVG.
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {0} {0}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="white"/><path d="{path}" fill="black"/></svg>"#,
        width + 8
    );
    Ok(format!(
        "data:image/svg+xml;base64,{}",
        STANDARD.encode(svg)
    ))
}

#[derive(Clone)]
pub(crate) struct Api {
    client: reqwest::Client,
    base: reqwest::Url,
    token: Option<String>,
}
#[derive(Debug, PartialEq)]
pub(crate) enum ApiError {
    Network,
    Protocol,
    SessionExpired,
}
pub(crate) struct Confirmation {
    pub token: String,
    pub bot_id: String,
    pub owner_id: String,
    pub base: String,
}
pub(crate) fn confirmation(value: &Value, fallback_base: &str) -> Result<Confirmation, String> {
    let field = |name: &str, max: usize| {
        value[name]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.len() <= max)
            .map(str::to_owned)
            .ok_or_else(|| "扫码信息不完整，请重新扫码".to_string())
    };
    if value["status"] != "confirmed" {
        return Err("扫码尚未确认".into());
    }
    let base = if value.get("baseurl").is_none() {
        validate_base(fallback_base)?.to_string()
    } else {
        validate_base(&field("baseurl", 2048)?)?.to_string()
    };
    Ok(Confirmation {
        token: field("bot_token", 16384)?,
        bot_id: field("ilink_bot_id", 256)?,
        owner_id: field("ilink_user_id", 256)?,
        base,
    })
}
pub(crate) fn update_batch(value: &Value) -> Result<(&[Value], &str), ApiError> {
    let messages = match value.get("msgs") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(items)) => items.as_slice(),
        _ => return Err(ApiError::Protocol),
    };
    let cursor = match value.get("get_updates_buf") {
        None | Some(Value::Null) => "",
        Some(Value::String(cursor)) => cursor.as_str(),
        _ => return Err(ApiError::Protocol),
    };
    Ok((messages, cursor))
}
impl Api {
    pub(crate) fn new(base: &str, token: Option<String>) -> Result<Self, String> {
        let base = validate_base(base)?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(40))
            .build()
            .map_err(|_| "无法初始化微信连接")?;
        Ok(Self {
            client,
            base,
            token,
        })
    }
    fn app_headers(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .header("iLink-App-Id", "bot")
            .header("iLink-App-ClientVersion", "132105")
    }
    async fn parse(
        &self,
        response: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<Value, ApiError> {
        let mut response = response.map_err(|_| ApiError::Network)?;
        if !response.status().is_success() {
            return Err(if response.status().as_u16() == 401 {
                ApiError::SessionExpired
            } else {
                ApiError::Network
            });
        }
        let mut data = vec![];
        while let Some(chunk) = response.chunk().await.map_err(|_| ApiError::Network)? {
            if data.len() + chunk.len() > 1_048_576 {
                return Err(ApiError::Protocol);
            }
            data.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&data).map_err(|_| ApiError::Protocol)?;
        if !value.is_object() {
            return Err(ApiError::Protocol);
        }
        if value["ret"].as_i64() == Some(-14) || value["errcode"].as_i64() == Some(-14) {
            return Err(ApiError::SessionExpired);
        }
        for field in ["ret", "errcode"] {
            if let Some(code) = value.get(field) {
                let code = code.as_i64().ok_or(ApiError::Protocol)?;
                if code != 0 {
                    return Err(ApiError::Protocol);
                }
            }
        }
        Ok(value)
    }
    async fn post(
        &self,
        path: &str,
        mut body: Value,
        authenticated: bool,
    ) -> Result<Value, ApiError> {
        let url = self.base.join(path).map_err(|_| ApiError::Protocol)?;
        let random = uuid::Uuid::new_v4();
        let uin = u32::from_le_bytes(
            random.as_bytes()[..4]
                .try_into()
                .map_err(|_| ApiError::Protocol)?,
        );
        let mut request = self
            .app_headers(self.client.post(url))
            .header("AuthorizationType", "ilink_bot_token")
            .header("X-WECHAT-UIN", STANDARD.encode(uin.to_string()));
        if authenticated {
            request = request.bearer_auth(self.token.as_ref().ok_or(ApiError::SessionExpired)?);
            body["base_info"] = json!({"channel_version":CHANNEL_VERSION,"bot_agent":concat!("MSLDesktop/",env!("CARGO_PKG_VERSION"))});
        }
        self.parse(request.json(&body).send().await).await
    }
    pub(crate) async fn request_qr(&self) -> Result<Value, ApiError> {
        self.post(
            "ilink/bot/get_bot_qrcode?bot_type=3",
            json!({"local_token_list":[]}),
            false,
        )
        .await
    }
    pub(crate) async fn poll_qr(
        &self,
        qrcode: &str,
        verify: Option<&str>,
    ) -> Result<Value, ApiError> {
        let mut url = self
            .base
            .join("ilink/bot/get_qrcode_status")
            .map_err(|_| ApiError::Protocol)?;
        url.query_pairs_mut().append_pair("qrcode", qrcode);
        if let Some(code) = verify {
            url.query_pairs_mut().append_pair("verify_code", code);
        }
        self.parse(self.app_headers(self.client.get(url)).send().await)
            .await
    }
    pub(crate) async fn updates(&self, cursor: &str) -> Result<Value, ApiError> {
        self.post(
            "ilink/bot/getupdates",
            json!({"get_updates_buf":cursor}),
            true,
        )
        .await
    }
    pub(crate) async fn acknowledge(
        &self,
        owner: &str,
        context: &str,
        text: &str,
    ) -> Result<(), ApiError> {
        self.post("ilink/bot/sendmessage",json!({"msg":{"from_user_id":"","to_user_id":owner,"client_id":format!("msl-{}",uuid::Uuid::new_v4()),"message_type":2,"message_state":2,"context_token":context,"item_list":[{"type":1,"text_item":{"text":text}}]}}),true).await.map(|_|())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    #[test]
    fn only_https_official_origin_is_allowed() {
        assert!(validate_base("https://ilinkai.weixin.qq.com").is_ok());
        assert!(validate_base("https://ilinkai-sg.weixin.qq.com/").is_ok());
        for url in [
            "http://ilinkai.weixin.qq.com",
            "https://weixin.qq.com.evil.example",
            "https://evil.example",
            "https://user:pass@ilinkai.weixin.qq.com",
            "https://ilinkai.weixin.qq.com:8443",
            "https://ilinkai.weixin.qq.com/evil",
            "https://ilinkai.weixin.qq.com/?x=1",
        ] {
            assert!(validate_base(url).is_err(), "accepted {url}");
        }
    }
    #[test]
    fn qr_payload_is_encoded_as_pixels_never_interpolated() {
        let uri = qr_image("<script>alert('synthetic')</script>").unwrap();
        assert!(uri.starts_with("data:image/svg+xml;base64,"));
        let svg = String::from_utf8(
            base64::engine::general_purpose::STANDARD
                .decode(uri.split_once(',').unwrap().1)
                .unwrap(),
        )
        .unwrap();
        assert!(svg.contains("<path"));
        assert!(!svg.contains("script"));
        assert!(!svg.contains("synthetic"));
    }
    fn server(response: &str, status: u16) -> (String, std::sync::mpsc::Receiver<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let response = response.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        listener.set_nonblocking(true).unwrap();
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            while start.elapsed().as_secs() < 5 {
                if let Ok((mut stream, _)) = listener.accept() {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut request = vec![];
                    let mut bytes = [0u8; 4096];
                    loop {
                        let n = stream.read(&mut bytes).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        request.extend_from_slice(&bytes[..n]);
                        let text = String::from_utf8_lossy(&request);
                        if let Some((head, body)) = text.split_once("\r\n\r\n") {
                            let len = head
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length: ")
                                        .and_then(|n| n.parse::<usize>().ok())
                                })
                                .unwrap_or(0);
                            if body.len() >= len {
                                break;
                            }
                        }
                    }
                    let _ = tx.send(String::from_utf8(request).unwrap());
                    let _=write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",response.len());
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        (address, rx)
    }
    #[test]
    fn qr_uses_post_local_token_list_without_bearer() {
        let (base, rx) = server(
            r#"{"qrcode":"synthetic","qrcode_img_content":"synthetic-payload"}"#,
            200,
        );
        let value = tauri::async_runtime::block_on(Api::new(&base, None).unwrap().request_qr());
        assert!(value.is_ok());
        let request = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        assert!(request.starts_with("POST /ilink/bot/get_bot_qrcode?bot_type=3 "));
        let (head, body) = request.split_once("\r\n\r\n").unwrap();
        assert!(!head.to_ascii_lowercase().contains("authorization: bearer"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(body).unwrap(),
            serde_json::json!({"local_token_list":[]})
        );
    }
    #[test]
    fn poll_sends_cursor_with_auth_and_marks_expired_session() {
        let (base, rx) = server(
            r#"{"ret":-14,"errcode":-14,"errmsg":"synthetic-private-server-detail"}"#,
            200,
        );
        let result = tauri::async_runtime::block_on(
            Api::new(&base, Some("synthetic-token".into()))
                .unwrap()
                .updates("cursor-example"),
        );
        assert_eq!(result.unwrap_err(), ApiError::SessionExpired);
        let request = rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
        let (head, body) = request.split_once("\r\n\r\n").unwrap();
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: bearer synthetic-token"));
        let body: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["get_updates_buf"], "cursor-example");
        assert_eq!(body["base_info"]["channel_version"], "2.4.9");
    }
    #[test]
    fn pairing_requires_owner_token_and_official_destination() {
        let valid = json!({"status":"confirmed","bot_token":"synthetic-token","ilink_bot_id":"synthetic-bot","ilink_user_id":"synthetic-owner","baseurl":"https://ilinkai.weixin.qq.com"});
        assert!(confirmation(&valid, DEFAULT_BASE).is_ok());
        for field in ["bot_token", "ilink_bot_id", "ilink_user_id", "baseurl"] {
            let mut value = valid.clone();
            value[field] = Value::Null;
            assert!(confirmation(&value, DEFAULT_BASE).is_err());
        }
        let mut evil = valid;
        evil["baseurl"] = json!("https://evil.example");
        assert!(confirmation(&evil, DEFAULT_BASE).is_err());
    }
    #[test]
    fn optional_update_fields_and_pairing_origin_use_safe_defaults() {
        assert_eq!(update_batch(&json!({})).unwrap(), (&[][..], ""));
        assert_eq!(
            update_batch(&json!({"msgs":null,"get_updates_buf":null})).unwrap(),
            (&[][..], "")
        );
        for invalid in [json!({"msgs":"bad"}), json!({"get_updates_buf":123})] {
            assert!(update_batch(&invalid).is_err());
        }
        let confirmed = json!({"status":"confirmed","bot_token":"synthetic-token","ilink_bot_id":"synthetic-bot","ilink_user_id":"synthetic-owner"});
        assert_eq!(
            confirmation(&confirmed, DEFAULT_BASE).unwrap().base,
            "https://ilinkai.weixin.qq.com/"
        );
        assert!(confirmation(&confirmed, "https://evil.example").is_err());
    }
    #[test]
    fn null_error_code_is_rejected_like_official_monitor() {
        let (base, _rx) = server(
            r#"{"ret":0,"errcode":null,"msgs":[],"get_updates_buf":"next"}"#,
            200,
        );
        let result = tauri::async_runtime::block_on(
            Api::new(&base, Some("synthetic-token".into()))
                .unwrap()
                .updates(""),
        );
        assert_eq!(result.unwrap_err(), ApiError::Protocol);
    }
    #[test]
    fn expired_errcode_takes_priority_over_generic_ret() {
        let (base, _rx) = server(r#"{"ret":-1,"errcode":-14}"#, 200);
        let result = tauri::async_runtime::block_on(
            Api::new(&base, Some("synthetic-token".into()))
                .unwrap()
                .updates(""),
        );
        assert_eq!(result.unwrap_err(), ApiError::SessionExpired);
    }
}
