use dioxus::prelude::*;
use dioxus_desktop::{tao::platform::macos::WindowBuilderExtMacOS, WindowBuilder};
use google_gmail1::api::Message;
mod activites;
mod mail;

fn main() {
    // include tailwind from cdn
    static CUSTOM_HEAD: &str = r#"
    <script src="https://cdn.tailwindcss.com"></script>
    <style type="text/css">
        html, body {
            height: 100%;
            margin: 0;
            overscroll-behavior-y: none;
            overscroll-behavior-x: none;
            overflow: hidden;
        }
        #main, #bodywrap {
            height: 100%;
            margin: 0;
            overscroll-behavior-x: none;
            overscroll-behavior-y: none;
        }
    </style>
"#;

    dioxus_desktop::launch_cfg(
        app,
        dioxus_desktop::Config::default()
            .with_custom_head(CUSTOM_HEAD.into())
            .with_window(
                WindowBuilder::new()
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_titlebar_transparent(true)
                    .with_maximized(true),
            ),
    );
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div { class: "flex flex-row rounded-lg overflow-hidden", id: "bodywrap",
            side_bar {}
            message_list {}
            preview_message {}
        }
    })
}

fn side_bar(cx: Scope) -> Element {
    cx.render(rsx! {
        div { class: "bg-gray-200 p-4 w-40", opacity: "0.95",
            h1 { "Sidebar" }
            ul { class: "list-disc truncate",
                li { "Inbox" }
                li { "Sent" }
                li { "Drafts" }
                li { "Trash" }
            }
        }
    })
}

fn message_list(cx: Scope<'_>) -> Element {
    let messages = use_state(cx, || {
        // check if the index already exists
        // if it does, load it and return it
        // todo: update the cache
        match std::fs::read_to_string("data/sensitive/index.json").map(|s| serde_json::from_str(&s))
        {
            Ok(Ok(index)) => return index,
            _ => log::info!("Index not found, downloading messages"),
        }

        Vec::<google_gmail1::api::Message>::new()
    });

    cx.use_hook(|| {
        if messages.is_empty() {
            to_owned![messages];
            cx.spawn(async move {
                let new_messages = mail::download_recent_messages().await;
                messages.set(new_messages);
            });
        }
    });

    cx.render(rsx! {
        div { class: "flex-col flex-grow w-1/2 bg-white",
            div { class: "flex-grow h-full",
                div { class: "p-2 bg-white border-b border-gray-400 flex flex-row justify-between items-center h-12",
                    // Helpful display info on the left of the row
                    div { class: "flex flex-col",
                        h1 { class: "font-bold text-sm", "Important -- Google " }
                        h3 { class: "text-xs", "2,5438 messages, 100 unread" }
                    }

                    // Filters for Primary, Social, Promotions, Updates, Forums
                    FilterGroup {}
                }

                div { class: "h-full flex flex-col items-stretch",
                    div { class: "flex flex-row flex-auto min-h-0",
                        div { class: "flex flex-col items-stretch min-h-0 overflow-x-hidden", style: "flex: 0 0 100%;",
                            div { class: "bg-gray-200 text-bold font-sm flex flex-row border-b border-gray-400",
                                div { class: "flex-1 overflow-hidden ml-4", "From" }
                                div { class: "flex-1 overflow-hidden ml-4", "Snippet" }
                                div { class: "flex-1 overflow-hidden ml-4", "Date" }
                            }
                            div { class: "flex-initial min-h-0 overflow-y-auto",
                                messages.iter().map(|msg| rsx!(
                                    message_li { message: msg }
                                ))
                            }
                        }
                    }
                }
            }
        }
    })
}

#[inline_props]
fn message_li<'a>(cx: Scope<'a>, message: &'a Message) -> Element {
    let snippet = cx.use_hook(|| {
        //
        let raw = message.snippet.as_ref().unwrap();
        let mut out = String::new();
        html_escape::decode_html_entities_to_string(raw, &mut out);
        out
    });

    let (name, email) = cx.use_hook(|| {
        let make = || {
            let headers = message.payload.as_ref()?.headers.as_ref()?;
            let value = headers.iter().find(|h| h.name.as_deref() == Some("From"))?;
            let raw = value.value.as_ref().cloned();

            // split the email from the name
            let (from, email) = raw
                .as_deref()
                .and_then(|s| s.split_once('<'))
                .map(|(from, email)| (from.trim(), email.trim_end_matches('>')))
                .unwrap_or_default();

            let mut out = String::new();
            html_escape::decode_html_entities_to_string(from, &mut out);

            Some((out.to_string(), email.to_string()))
        };

        make().unwrap_or_default()
    });

    cx.render(rsx! {
        div { class: "text-bold font-sm overflow-hidden truncate flex flex-row cursor-default",
            div { class: "flex-1 overflow-hidden ml-4", "{name}" }
            div { class: "flex-1 overflow-hidden ml-4", "{snippet}" }
            div { class: "flex-1 overflow-hidden ml-4", "Date" }
        }
    })
}

fn preview_message(cx: Scope) -> Element {
    cx.render(rsx!(
        //
        div { class: "flex flex-col bg-gray-500 flex-grow",
            div { class: "flex bg-gray-100 p-4 h-12", "toolbar goes here" }
            div { class: "m-auto", "no message selected" }
        }
    ))
}

fn FilterGroup(cx: Scope) -> Element {
    let filters = &[
        ("Primary", "primary"),
        ("Social", "social"),
        ("Promotions", "promotions"),
        ("Updates", "updates"),
        ("Forums", "forums"),
    ];

    cx.render(rsx! {
        ul { class: "flex flex-row",
            filters.iter().map(|(name, id)| rsx!(
                li { class: "flex flex-1 mx-1 text-xs",
                    input { class: "hidden peer", id: "filter-{id}", r#type: "radio", name: "hosting",  value: "filter-{id}", }
                    label {
                        class: "p-1 text-gray-500 bg-white rounded-lg border border-gray-200 cursor-pointer dark:hover:text-gray-300 dark:border-gray-700 dark:peer-checked:text-blue-500 peer-checked:border-blue-600 peer-checked:text-blue-600 hover:text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:bg-gray-800 dark:hover:bg-gray-700",
                        r#for: "filter-{id}",
                        div { class: "block", "{name}" }
                    }
                }
            ))
        }
    })
}
