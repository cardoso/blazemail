use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use google_gmail1::{
    api::ListMessagesResponse,
    oauth2::{
        authenticator::{Authenticator, AuthenticatorBuilder},
        authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate},
        authorized_user::AuthorizedUserSecret,
        storage::{TokenInfo, TokenStorage},
        ApplicationSecret, AuthorizedUserAuthenticator, InstalledFlowAuthenticator,
    },
    Gmail,
};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use tokio::task::JoinSet;

static GMAIL_SCOPES: &[&str] = &[
    "https://mail.google.com/",                         // email
    "https://www.googleapis.com/auth/userinfo.email",   // email address
    "https://www.googleapis.com/auth/userinfo.profile", // G+ profile
    "https://www.googleapis.com/auth/contacts",         // contacts
    "https://www.googleapis.com/auth/calendar",         // calendar
];

struct OpenBrowserDelegate;
impl InstalledFlowDelegate for OpenBrowserDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            webbrowser::open(url).unwrap();
            Ok(String::new())
        })
    }
}

async fn make_client() -> Gmail<HttpsConnector<HttpConnector>> {
    // yes, you do indeed distribute your client secret with your app
    // it's fine to do this, but please don't abuse our API access <3
    let secret = google_gmail1::oauth2::read_application_secret("client_secret.json")
        .await
        .unwrap();

    let auth = InstalledFlowAuthenticator::builder(
        secret,
        google_gmail1::oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokencache.json")
    .flow_delegate(Box::new(OpenBrowserDelegate))
    .build()
    .await
    .unwrap();

    let client = hyper::Client::builder().build(
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    );

    Gmail::new(client, auth)
}

#[tokio::test]
async fn list_messages_works() {
    let hub = make_client().await;

    let (result, messages) = hub
        .users()
        .messages_list("jkelleyrtp@gmail.com")
        .add_scope("https://mail.google.com/")
        // .add_scope("https://www.googleapis.com/auth/gmail.metadata")
        .doit()
        .await
        .unwrap();

    std::fs::write(
        "data/messages/sensitive.json",
        serde_json::to_string_pretty(&messages).unwrap(),
    )
    .unwrap();

    dbg!(result);
}

#[tokio::test]
async fn load_next_page() {
    let hub = make_client().await;

    let (result, messages) = hub
        .users()
        .messages_list("jkelleyrtp@gmail.com")
        .page_token("06141333176801119710")
        .add_scope("https://mail.google.com/")
        // .add_scope("https://www.googleapis.com/auth/gmail.metadata")
        .doit()
        .await
        .unwrap();

    std::fs::write(
        "data/messages2/sensitive.json",
        serde_json::to_string_pretty(&messages).unwrap(),
    )
    .unwrap();
}

pub async fn download_recent_messages() -> Vec<google_gmail1::api::Message> {
    let messages = std::fs::read_to_string("data/sensitive/messages2.json").unwrap();
    let messages = serde_json::from_str::<ListMessagesResponse>(&messages).unwrap();

    let hub = make_client().await;

    let mut set: JoinSet<google_gmail1::Result<google_gmail1::api::Message>> = JoinSet::new();

    let hubbed = Arc::new(hub);
    for msg in messages.messages.unwrap().iter().take(500).cloned() {
        let hub = hubbed.clone();
        set.spawn(async move {
            let id = msg.id.as_ref().unwrap();

            let res = hub
                .users()
                .messages_get("jkelleyrtp@gmail.com", id)
                .add_scope("https://mail.google.com/")
                .doit()
                .await?;

            Ok(res.1)
        });
    }

    let mut messages = vec![];

    while let Some(Ok(msg)) = set.join_next().await {
        if let Ok(msg) = msg {
            messages.push(msg)
        }
    }

    messages.sort_by(|l, r| l.internal_date.cmp(&r.internal_date).reverse());

    // save
    std::fs::File::create("data/sensitive/index.json").unwrap();
    std::fs::write(
        "data/sensitive/index.json",
        serde_json::to_string_pretty(&messages).unwrap(),
    )
    .unwrap();

    messages
}

#[tokio::test]
async fn read_messages() {
    let messages = std::fs::read_to_string("data/sensitive/messages2.json").unwrap();
    let messages = serde_json::from_str::<ListMessagesResponse>(&messages).unwrap();

    let hub = make_client().await;

    let mut set: JoinSet<google_gmail1::Result<google_gmail1::api::Message>> = JoinSet::new();

    let hubbed = Arc::new(hub);
    for msg in messages.messages.unwrap().iter().take(200).cloned() {
        let hub = hubbed.clone();
        set.spawn(async move {
            let id = msg.id.as_ref().unwrap();

            let res = hub
                .users()
                .messages_get("jkelleyrtp@gmail.com", id)
                .add_scope("https://mail.google.com/")
                .doit()
                .await?;

            Ok(res.1)
        });
    }

    let mut messages = vec![];

    while let Some(Ok(msg)) = set.join_next().await {
        if let Ok(msg) = msg {
            messages.push(msg)
        }
    }

    messages.sort_by(|l, r| l.internal_date.cmp(&r.internal_date).reverse());

    for msg in messages {
        println!(
            "{:?} - {}",
            msg.internal_date,
            msg.snippet.unwrap_or_default()
        );
    }
}
