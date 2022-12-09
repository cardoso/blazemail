use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    SmtpTransport, Tokio1Executor, Transport,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub enum MailAction {
    UndoSend,
    Send(SendAction),
}

pub struct SendAction {
    body: lettre::Message,
}

async fn main_loop(mut cx: UnboundedReceiver<MailAction>) {
    // load the index from disk

    let last_refreshed = std::time::Instant::now();

    loop {
        tokio::select! {
            msg = cx.recv() => {
                //
            }
        }
    }
}

async fn send_message(email: lettre::Message) {
    const USERNAME: &str = "jkelleyrtp@gmail.com";
    const PASSWORD: &str = "rqgvegctdaproiue";

    let creds = Credentials::new(USERNAME.into(), PASSWORD.into());

    // Open a remote connection to gmail
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    match mailer.send(email).await {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => panic!("Could not send email: {:?}", e),
    }
}
