use async_trait::async_trait;
use godwit_core::{AppConfig, MailConfig, MailTls};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tera::Tera;

#[async_trait]
pub trait SendEmail: Send + Sync {
    async fn send(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), MailError>;
}

#[derive(Debug)]
pub enum MailError {
    Build(String),
    Transport(String),
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailError::Build(s) => write!(f, "mail build error: {s}"),
            MailError::Transport(s) => write!(f, "mail transport error: {s}"),
        }
    }
}

impl std::error::Error for MailError {}

pub struct Mailer {
    mail: MailConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
    templates: Tera,
}

impl Mailer {
    pub fn build(config: &AppConfig) -> Result<Option<Self>, MailError> {
        let mail = match &config.auth.mail {
            Some(m) => m.clone(),
            None => return Ok(None),
        };
        let mut builder = match mail.tls {
            MailTls::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&mail.host)
                .map_err(|e| MailError::Build(e.to_string()))?,
            MailTls::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&mail.host)
                .map_err(|e| MailError::Build(e.to_string()))?,
            MailTls::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&mail.host),
        };
        builder = builder.port(mail.port);
        if let (Some(u), Some(p)) = (&mail.username, &mail.password) {
            builder = builder.credentials(Credentials::new(u.clone(), p.clone()));
        }
        let transport = builder.build();
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("reset_password.html", include_str!("../assets/email/reset_password.html")),
            ("reset_password.txt", include_str!("../assets/email/reset_password.txt")),
            ("password_changed.html", include_str!("../assets/email/password_changed.html")),
            ("password_changed.txt", include_str!("../assets/email/password_changed.txt")),
        ])
        .map_err(|e| MailError::Build(e.to_string()))?;
        Ok(Some(Self { mail, transport, templates: tera }))
    }

    pub fn from(&self) -> Mailbox {
        self.mail.from.parse().expect("valid from mailbox")
    }

    pub fn render(&self, name: &str, ctx: &tera::Context) -> (String, String) {
        let html = self.templates.render(&format!("{name}.html"), ctx).unwrap_or_default();
        let text = self.templates.render(&format!("{name}.txt"), ctx).unwrap_or_default();
        (html, text)
    }
}

#[async_trait]
impl SendEmail for Mailer {
    async fn send(&self, to: &str, subject: &str, html: &str, text: &str) -> Result<(), MailError> {
        let email = Message::builder()
            .from(self.from())
            .to(to.parse::<lettre::Address>().map_err(|e| MailError::Build(e.to_string()))?.into())
            .subject(subject)
            .multipart(lettre::message::MultiPart::alternative_plain_html(
                text.to_string(), html.to_string(),
            ))
            .map_err(|e| MailError::Build(e.to_string()))?;
        self.transport.send(email).await.map_err(|e| MailError::Transport(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tera::Tera;

    #[test]
    fn reset_password_template_renders_link() {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "reset_password.html",
            include_str!("../assets/email/reset_password.html"),
        )
        .unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("reset_link", "https://example.com/reset?token=abc");
        let html = tera.render("reset_password.html", &ctx).unwrap();
        assert!(html.contains("example.com"));
        assert!(html.contains("token=abc"));
    }

    #[test]
    fn password_changed_template_renders_brand() {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "password_changed.html",
            include_str!("../assets/email/password_changed.html"),
        )
        .unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("brand", "Godwit");
        let html = tera.render("password_changed.html", &ctx).unwrap();
        assert!(html.contains("Godwit"));
    }
}
