# Channel Integrations 🟡 BETA

This guide covers integrating messaging channels with General Bots, focusing on WhatsApp Business API integration using Twilio-purchased phone numbers.

## Overview

General Bots supports multiple messaging channels through a unified API. This section focuses on WhatsApp Business API, the most widely used business messaging platform globally.

## Supported Channels

| Channel | Status | Config Keys |
|---------|--------|-------------|
| WhatsApp | ✅ Production Ready | `whatsapp-api-key`, `whatsapp-phone-number-id` |
| Twilio SMS | ✅ Production Ready | `twilio-account-sid`, `twilio-auth-token` |
| Instagram | ✅ Production Ready | `instagram-access-token`, `instagram-page-id` |
| Microsoft Teams | ✅ Production Ready | `teams-app-id`, `teams-app-password` |

## Adding a Channel

A channel is a crate in `botserver/crates/` that implements **one** contract:

```rust
// botlib/src/traits/core.rs
pub trait ChannelAdapter: Send + Sync + Debug { .. }
```

There is a single entry point, `botlib::traits::ChannelAdapter`. The web and
voice runtime adapters and the Telegram, Instagram and Teams adapters all
implement it, so anything holding `Arc<dyn ChannelAdapter>` can drive any
transport. Per-channel crates re-export the trait
(`pub use botlib::traits::ChannelAdapter;`) instead of declaring their own.

A new channel therefore:

1. implements `botlib::traits::ChannelAdapter` for its adapter struct;
2. implements `channel_type` (stable id, e.g. `"matrix"`), `display_name`,
   `is_configured` and — when the transport can deliver a full bot response —
   `send_bot_response`;
3. inherits the defaults for `send_message` (synchronous delivery), the
   connection-tracking methods and the voice-session methods. The defaults
   return an explicit "not supported" error instead of pretending to work;
4. mounts its routes on the `Router` the botserver builds, with the
   credentials read from Vault under `secret/gbo/{org}/{branch}/{bot}/{channel}`.

Inbound webhooks normalise their payload through `receive_message`; anything
the channel cannot map to chat text returns `Ok(None)`.

## WhatsApp Business Integration

The most popular channel for business messaging. Complete integration guide: [WhatsApp Quick Start](./whatsapp-quick-start.md)

### Quick Setup (5 minutes)

1. **Purchase a phone number from Twilio**
   ```bash
   # Twilio Console > Phone Numbers > Buy a Number
   # Select: Voice capability (required for verification)
   # Example: +553322980098
   ```

2. **Create Meta App with WhatsApp**
   ```bash
   # https://developers.facebook.com/apps/
   # Create App > Business > Add WhatsApp product
   ```

3. **Configure credentials in `config.csv`**
   ```csv
   whatsapp-enabled,true
   whatsapp-api-key,EAAQyour_api_key_here
   whatsapp-phone-number-id,YOUR_PHONE_NUMBER_ID
   whatsapp-business-account-id,YOUR_WABA_ID
   whatsapp-webhook-verify-token,4qIogZadggQ.BEoMeciXIdl_MlkV_1DTx8Z_i0bYPxtSJwKSbH0FKlY
   whatsapp-application-id,YOUR_APP_ID
   ```

### BASIC Keywords for WhatsApp

```basic
REM Send a message
SEND WHATSAPP TO "+5511999999999" WITH "Hello from General Bots!"

REM Handle incoming messages
ON WHATSAPP MESSAGE RECEIVED
  LET SENDER$ = GET WHATSAPP SENDER NUMBER
  LET MESSAGE$ = GET WHATSAPP MESSAGE BODY
  
  REM Echo message back
  SEND WHATSAPP TO SENDER$ WITH "You said: " + MESSAGE$
END ON
```

### Credential Reference

| Credential | Format | Example | Purpose |
|------------|--------|---------|---------|
| Access Token | `EAAQ...` | `EAAQyour_api_key_here` | API authentication |
| Phone Number ID | 16 digits | `YOUR_PHONE_NUMBER_ID` | Message sending endpoint |
| WABA ID | 15 digits | `YOUR_WABA_ID` | Business account identifier |
| Verify Token | Custom string | `4qIogZadggQ.BEoMeci...` | Webhook security |
| Application ID | 15 digits | `YOUR_APP_ID` | App identifier |

### Phone Number Verification

Twilio numbers require **voice call verification** (not SMS):

1. **Configure Twilio webhook** to capture verification calls
   ```xml
   <!-- TwiML for voice handling -->
   <?xml version="1.0" encoding="UTF-8"?>
   <Response>
     <Gather action="https://twimlets.com/voicemail?Email=your@email.com">
       <Say voice="alice">Please enter your verification code.</Say>
     </Gather>
   </Response>
   ```

2. **In Meta Business Suite**: Select "Phone Call" verification method
3. **Enter the 6-digit code** received via email
4. **Verification complete** - number ready for WhatsApp

See: [Webhook Configuration Guide](./whatsapp-webhooks.md)

## Advanced Configuration

### Message Templates

For business-initiated messages outside the 24-hour window:

```javascript
// Send template message
POST https://graph.facebook.com/v18.0/YOUR_PHONE_NUMBER_ID/messages
{
  "messaging_product": "whatsapp",
  "to": "5511999999999",
  "type": "template",
  "template": {
    "name": "hello_world",
    "language": { "code": "pt_BR" }
  }
}
```

### Rate Limiting

WhatsApp enforces rate limits per tier:

| Tier | Messages/Day | Messages/Second |
|------|--------------|-----------------|
| Tier 1 | 1,000 | 1 |
| Tier 2 | 10,000 | 5 |
| Tier 3 | 100,000 | 50 |
| Tier 4 | Unlimited | 1,000 |

Implement rate limiting in your bot:

```basic
REM Simple rate limiting
LET LAST_SENT = 0
SUB SEND WHATSAPP WITH LIMIT TO NUMBER$, MESSAGE$
  LET NOW = TIMER
  IF NOW - LAST_SENT < 1 THEN
    WAIT 1 - (NOW - LAST_SENT)
  END IF
  SEND WHATSAPP TO NUMBER$ WITH MESSAGE$
  LAST_SENT = TIMER
END SUB
```

### Webhook Security

Always verify webhook signatures:

```javascript
// Node.js signature verification
const crypto = require('crypto');

function verifySignature(payload, signature, appSecret) {
  const expected = 'sha256=' + 
    crypto.createHmac('sha256', appSecret)
      .update(payload)
      .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expected)
  );
}
```

## Complete Documentation

For detailed guides and examples:

- **[WhatsApp Quick Start Guide](./whatsapp-quick-start.md)** - 30-minute setup walkthrough
- **[Webhook Configuration](./whatsapp-webhooks.md)** - Detailed webhook setup for Twilio and Meta
- **[Code Examples](./whatsapp-examples.md)** - Examples in BASIC, Node.js, and Python
- **[Troubleshooting Guide](./whatsapp-troubleshooting.md)** - Common issues and solutions
- **[Quick Reference](./whatsapp-quick-reference.md)** - Commands, configs, and snippets

## Other Channels

### Twilio SMS

Simple SMS integration using Twilio:

```csv
# config.csv
twilio-account-sid,ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
twilio-auth-token,your_auth_token_here
twilio-from-number,+15551234567
```

```basic
REM Send SMS
SEND SMS TO "+5511999999999" WITH "Hello via SMS!"
```

### Instagram Direct Messages

Connect Instagram messaging:

```csv
# config.csv
instagram-access-token,EAAxxxx...
instagram-page-id,123456789012345
```

```basic
REM Send Instagram DM
SEND INSTAGRAM TO "1234567890" WITH "Hello via Instagram!"
```

## Configuration Template

Complete channel configuration example:

```csv
# config.csv

# WhatsApp Business (Primary channel)
whatsapp-enabled,true
whatsapp-api-key,EAAQyour_api_key_here
whatsapp-phone-number-id,YOUR_PHONE_NUMBER_ID
whatsapp-business-account-id,YOUR_WABA_ID
whatsapp-webhook-verify-token,4qIogZadggQ.BEoMeciXIdl_MlkV_1DTx8Z_i0bYPxtSJwKSbH0FKlY

# Twilio SMS (Backup channel)
twilio-enabled,false
twilio-account-sid,ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
twilio-auth-token,your_auth_token_here
twilio-from-number,+15551234567

# Instagram (Social channel)
instagram-enabled,false
instagram-access-token,EAAxxxx...
instagram-page-id,123456789012345
```

## Troubleshooting

### Common Issues

**Issue: Phone number verification fails**
- **Solution**: Ensure "Phone Call" verification is selected (not SMS)
- **Solution**: Verify Twilio webhook is configured correctly
- See: [Troubleshooting Guide](./whatsapp-troubleshooting.md)

**Issue: Messages not sending**
- **Solution**: Check access token validity
- **Solution**: Verify phone number format: `5511999999999` (no +, no spaces)
- **Solution**: Ensure webhook is subscribed to "messages" field

**Issue: Rate limit errors**
- **Solution**: Implement rate limiting in your bot
- **Solution**: Use message queues for bulk sending
- See: [Code Examples](./whatsapp-examples.md)

## Best Practices

1. **Never hardcode credentials** - Always use `config.csv`
2. **Implement retry logic** - Handle API failures gracefully
3. **Monitor rate limits** - Respect platform limits
4. **Secure webhooks** - Verify all incoming requests
5. **Test thoroughly** - Use ngrok for local testing
6. **Log everything** - Track message delivery and errors
7. **Use templates** - Pre-approved templates for business-initiated messages
8. **Handle errors** - Provide user-friendly error messages

## Support

- **Documentation**: [Full guide](./whatsapp-quick-start.md)
- **Examples**: [Code samples](./whatsapp-examples.md)
- **Community**: [General Bots Discord](https://discord.gg/general-bots)
- **Meta Docs**: [WhatsApp Business API](https://developers.facebook.com/docs/whatsapp/)
- **Twilio Docs**: [Twilio WhatsApp](https://www.twilio.com/docs/whatsapp)

## Next Steps

1. Complete the [Quick Start Guide](./whatsapp-quick-start.md)
2. Set up webhooks using [Webhook Configuration](./whatsapp-webhooks.md)
3. Explore [Code Examples](./whatsapp-examples.md) for your use case
4. Configure monitoring and error handling
5. Test with your team before launching to users

For configuration of other services (LLM providers, databases, etc.), see [Appendix B: External Services](./README.md).