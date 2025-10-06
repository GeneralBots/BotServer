use anyhow::Result;
use jmap_client::{
    client::{Client, Credentials},
    core::query::Filter,
    email::{self, Property},
    mailbox::{self, Role},
};

#[tokio::test]
async fn test_successful_email_list() -> Result<()> {
    // JMAP server configuration
    

    // 1. Authenticate with JMAP server
    let client = Client::new()
        .credentials(("test@", ""))
        .connect("https://mail/jmap/")
        .await
        .unwrap();

    let inbox_id = client
        .mailbox_query(
            mailbox::query::Filter::role(Role::Inbox).into(),
            None::<Vec<_>>,
        )
        .await
        .unwrap()
        .take_ids()
        .pop()
        .unwrap();

    let email_id = client
        .email_query(
            Filter::and([
                //            email::query::Filter::subject("test"),
                email::query::Filter::in_mailbox(&inbox_id),
                //          email::query::Filter::has_keyword("$draft"),
            ])
            .into(),
            [email::query::Comparator::from()].into(),
        )
        .await
        .unwrap()
        .take_ids()
        .pop()
        .unwrap();

    // Fetch message
    let email = client
        .email_get(
            &email_id,
            [Property::Subject, Property::Preview, Property::Keywords].into(),
        )
        .await
        .unwrap();

    Ok(())
}
