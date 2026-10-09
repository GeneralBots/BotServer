use super::*;

pub fn send_template_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let _state_clone = state.clone();
    let user_clone = user.clone();
    let user_clone2 = user.clone();
    let user_clone3 = user;

    engine.register_fn(
        "SEND_TEMPLATE",
        move |template: &str, recipient: &str, channel: &str| -> Map {
            trace!(
                "SEND_TEMPLATE called: template={}, recipient={}, channel={} by user {}",
                template,
                recipient,
                channel,
                user_clone.user_id
            );

            send_template_message(template, recipient, channel, None)
        },
    );

    let _state_clone2 = state.clone();

    engine.register_fn(
        "send_template",
        move |template: &str, recipient: &str, channel: &str| -> Map {
            trace!(
                "send_template called: template={}, recipient={}, channel={} by user {}",
                template,
                recipient,
                channel,
                user_clone2.user_id
            );

            send_template_message(template, recipient, channel, None)
        },
    );

    let _state_clone3 = state;

    engine.register_fn(
        "SEND_TEMPLATE",
        move |template: &str, recipient: &str, channel: &str, variables: Map| -> Map {
            trace!(
                "SEND_TEMPLATE called with variables: template={}, recipient={}, channel={} by user {}",
                template,
                recipient,
                channel,
                user_clone3.user_id
            );

            send_template_message(template, recipient, channel, Some(&variables))
        },
    );

    debug!("Registered SEND_TEMPLATE keyword");
}

pub fn send_template_to_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();
    let user_clone2 = user.clone();
    let user_clone3 = user;
    let _state_clone = state.clone();

    engine.register_fn(
        "SEND_TEMPLATE_TO",
        move |template: &str, recipients: Array, channel: &str| -> Map {
            trace!(
                "SEND_TEMPLATE_TO called: template={}, recipients={:?}, channel={} by user {}",
                template,
                recipients.len(),
                channel,
                user_clone.user_id
            );

            send_template_batch(template, &recipients, channel, None)
        },
    );

    let _state_clone2 = state.clone();

    engine.register_fn(
        "SEND_TEMPLATE_TO",
        move |template: &str, recipients: Array, channel: &str, variables: Map| -> Map {
            trace!(
                "SEND_TEMPLATE_TO called with variables: template={}, recipients={:?}, channel={} by user {}",
                template,
                recipients.len(),
                channel,
                user_clone2.user_id
            );

            send_template_batch(template, &recipients, channel, Some(&variables))
        },
    );

    let _state_clone3 = state;

    engine.register_fn(
        "BULK_SEND",
        move |template: &str, recipients: Array, channel: &str| -> Map {
            trace!(
                "BULK_SEND called: template={}, recipients={:?}, channel={} by user {}",
                template,
                recipients.len(),
                channel,
                user_clone3.user_id
            );

            send_template_batch(template, &recipients, channel, None)
        },
    );

    debug!("Registered SEND_TEMPLATE_TO keyword");
}

pub fn create_template_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();
    let user_clone2 = user.clone();
    let user_clone3 = user;
    let _state_clone = state.clone();

    engine.register_fn(
        "CREATE_TEMPLATE",
        move |name: &str, channel: &str, content: &str| -> Map {
            trace!(
                "CREATE_TEMPLATE called: name={}, channel={} by user {}",
                name,
                channel,
                user_clone.user_id
            );

            create_message_template(name, channel, None, content)
        },
    );

    let _state_clone2 = state.clone();

    engine.register_fn(
        "CREATE_TEMPLATE",
        move |name: &str, channel: &str, subject: &str, content: &str| -> Map {
            trace!(
                "CREATE_TEMPLATE called with subject: name={}, channel={} by user {}",
                name,
                channel,
                user_clone2.user_id
            );

            create_message_template(name, channel, Some(subject), content)
        },
    );

    let _state_clone3 = state;

    engine.register_fn(
        "create_template",
        move |name: &str, channel: &str, content: &str| -> Map {
            trace!(
                "create_template called: name={}, channel={} by user {}",
                name,
                channel,
                user_clone3.user_id
            );

            create_message_template(name, channel, None, content)
        },
    );

    debug!("Registered CREATE_TEMPLATE keyword");
}

pub fn get_template_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let _state_clone = state.clone();
    let user_clone = user.clone();
    let user_clone2 = user.clone();
    let user_clone3 = user.clone();
    let user_clone4 = user;

    engine.register_fn("GET_TEMPLATE", move |name: &str| -> Map {
        trace!(
            "GET_TEMPLATE called: name={} by user {}",
            name,
            user_clone.user_id
        );

        get_message_template(name, None)
    });

    let _state_clone2 = state.clone();

    engine.register_fn("GET_TEMPLATE", move |name: &str, channel: &str| -> Map {
        trace!(
            "GET_TEMPLATE called: name={}, channel={} by user {}",
            name,
            channel,
            user_clone2.user_id
        );

        get_message_template(name, Some(channel))
    });

    let _state_clone3 = state.clone();

    engine.register_fn("get_template", move |name: &str| -> Map {
        trace!(
            "get_template called: name={} by user {}",
            name,
            user_clone3.user_id
        );

        get_message_template(name, None)
    });

    let _state_clone4 = state;

    engine.register_fn("LIST_TEMPLATES", move || -> Array {
        trace!("LIST_TEMPLATES called by user {}", user_clone4.user_id);

        debug!("Retrieving available message templates from database");
        let templates = vec![
            Dynamic::from("welcome"),
            Dynamic::from("order_confirmation"),
            Dynamic::from("password_reset"),
        ];
        debug!("Returned {} templates", templates.len());
        templates
    });

    debug!("Registered GET_TEMPLATE keyword");
}
