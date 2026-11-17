//! Sends transactional email through Resend's HTTP API. Built once at startup and
//! shared through AppState.

use std::time::Duration;

use serde::Serialize;

use crate::config::Config;

const RESEND_URL: &str = "https://api.resend.com/emails";

/// Embedded at compile time, so the binary needs no template files at runtime.
const LOGIN_CODE_HTML: &str = include_str!("templates/new-template.html");

#[derive(Debug, Clone)]
pub struct Mailer {
    http: reqwest::Client,
    api_key: String,
    from: String,
}

#[derive(Serialize)]
struct SendEmail<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: String,
    html: String,
}

impl Mailer {
    pub fn new(config: &Config) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build email HTTP client: {}", e))?;

        Ok(Self {
            http,
            api_key: config.resend_api_key.clone(),
            from: config.email_from.clone(),
        })
    }

    pub async fn send_login_code(&self, to: &str, code: &str) -> Result<(), String> {
        let body = SendEmail {
            from: &self.from,
            to: [to],
            subject: "Your Wage3 log-in code",
            // Plain-text fallback for clients that don't render HTML; it also helps spam scores.
            text: format!(
                "To continue setting up your Wage3 account, please verify your email \
                 address using the code below:\n\n\
                 {code}\n\n\
                 This code will expire in 10 minutes. If you didn't initiate this request, \
                 you can safely ignore this email.\n\n\
                 Best regards,\n\
                 The wage3 Team\n\n\
                 Questions? Reach us at hello@wage3.com"
            ),
            // `code` is six digits, so it needs no HTML escaping.
            html: LOGIN_CODE_HTML.replace("{{code}}", code),
        };

        let res = self
            .http
            .post(RESEND_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Resend request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let detail = res.text().await.unwrap_or_default();
            return Err(format!("Resend returned {}: {}", status, detail));
        }

        Ok(())
    }
}
