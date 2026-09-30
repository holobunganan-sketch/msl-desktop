//! Local-only, owner-bound Weixin capture bridge.
mod protocol;
mod runtime;
mod store;
pub use runtime::{
    begin_login, poll_login, set_enabled, start, status, submit_code, unlink, LoginView,
    WeixinRuntime, WeixinStatus,
};

#[cfg(test)]
mod tests;
