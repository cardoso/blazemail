mod open_browser_delegate;

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use google_gmail1::{
    api::ListMessagesResponse,
    oauth2::{
        authenticator::{Authenticator, AuthenticatorBuilder},
        authenticator_delegate::{DefaultInstalledFlowDelegate, InstalledFlowDelegate},
        authorized_user::AuthorizedUserSecret,
        storage::{TokenInfo, TokenStorage},
        ApplicationSecret, AuthorizedUserAuthenticator, InstalledFlowAuthenticator,
        InstalledFlowReturnMethod,
    },
    Gmail,
};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;
use tokio::task::JoinSet;

use self::open_browser_delegate::OpenBrowserDelegate;

static GMAIL_SCOPES: &[&str] = &[
    "https://mail.google.com/",                         // email
    "https://www.googleapis.com/auth/userinfo.email",   // email address
    "https://www.googleapis.com/auth/userinfo.profile", // G+ profile
    "https://www.googleapis.com/auth/contacts",         // contacts
    "https://www.googleapis.com/auth/calendar",         // calendar
];

async fn make_client() -> Gmail<HttpsConnector<HttpConnector>> {
    // yes, you do indeed distribute your client secret with your app
    // it's fine to do this, but please don't abuse our API access <3
    let secret = google_gmail1::oauth2::read_application_secret("data/client_secret.json")
        .await
        .unwrap();

    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("data/sensitive/tokencache.json") // todo: implement a secure custom token storage provider
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

    let hub = Gmail::new(client, auth);

    // do a quick query to make sure we're authenticated
    let (result, _) = hub
        .users()
        .get_profile("me")
        .add_scope("https://mail.google.com/")
        .doit()
        .await
        .unwrap();

    hub
}

#[tokio::test]
async fn list_messages_works() {
    let hub = make_client().await;

    let (result, messages) = hub
        .users()
        .messages_list("jkelleyrtp@gmail.com")
        .add_scope("https://mail.google.com/")
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
    let messages = std::fs::read_to_string("data/sensitive/messages.json").unwrap();
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
