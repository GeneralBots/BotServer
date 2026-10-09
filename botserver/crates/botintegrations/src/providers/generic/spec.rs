use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    Low,
    Medium,
    High,
}

/// Parameter types advertised on the chat surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Str,
    Integer,
    Json,
}

impl ParamKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Str => "string",
            Self::Integer => "integer",
            Self::Json => "json",
        }
    }
}

/// One declared action parameter (chat metadata and validation).
pub struct ParamSpec {
    pub name: &'static str,
    pub kind: ParamKind,
    pub required: bool,
}

/// Credential styles supported by the generic engine.
pub enum AuthStyle {
    /// `Authorization: Bearer <field>`.
    Bearer {
        token_field: &'static str,
    },
    /// `<header>: <field>` (for example `X-Api-Key`).
    ApiKeyHeader {
        header: &'static str,
        field: &'static str,
    },
    /// HTTP Basic with a templated username (`{email}/token`) and one
    /// password field. Template placeholders reference envelope fields.
    BasicTemplate {
        user_template: &'static str,
        password_field: &'static str,
    },
    /// HTTP Basic joining two fields with a separator; a missing second
    /// field renders as empty (Greenhouse `key:` style).
    BasicJoin {
        first_field: &'static str,
        separator: char,
        second_field: Option<&'static str>,
    },
    /// Credentials appended as query parameters (Trello `key`/`token`).
    QueryPairs {
        pairs: &'static [(&'static str, &'static str)],
    },
    /// Credential injected as a top-level JSON body field on every call;
    /// all declared parameters flatten into the same body (Canny style).
    BodyField {
        field: &'static str,
    },
    /// Multiple credential-backed headers in one style (Algolia application
    /// id + key).
    ApiKeyHeaders {
        pairs: &'static [(&'static str, &'static str)],
    },
    /// Bearer token plus fixed companion headers (API version pins such as
    /// `Notion-Version`).
    BearerHeaders {
        token_field: &'static str,
        headers: &'static [(&'static str, &'static str)],
    },
}

/// Origin resolution for providers whose host depends on the credential.
pub enum Origin {
    Static(&'static str),
    /// Zendesk: `https://{subdomain}.api.zendesk.com/api/v2`.
    ZendeskSubdomain,
    /// Mailchimp: data center suffix of the API key selects the host.
    MailchimpDataCenter,
    /// Credential-supplied base URL (`{value}` placeholder), restricted to
    /// https so a hostile envelope cannot downgrade transport.
    FromField {
        field: &'static str,
        pattern: &'static str,
    },
}

/// One executable provider action.
pub struct ActionSpec {
    pub key: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub summary: &'static str,
    /// Path placeholders substituted from params (URL-encoded).
    pub path_params: &'static [&'static str],
    /// Extra query parameters: `(request param name, params object key)`.
    pub query: &'static [(&'static str, &'static str)],
    /// Parameter carrying a JSON request body.
    pub body_param: Option<&'static str>,
    /// Optional top-level wrapper key for the JSON body (Zendesk `ticket`).
    pub body_wrapper: Option<&'static str>,
    pub risk: Risk,
    pub params: &'static [ParamSpec],
}

impl ActionSpec {
    pub(super) fn is_read(&self) -> bool {
        self.method == "GET"
    }
}

/// Static description of one generic REST provider.
pub struct ProviderSpec {
    pub slug: &'static str,
    pub origin: Origin,
    pub auth: AuthStyle,
    /// Keys mirrored exactly by [`ProviderSpec::action_keys`] (trait needs a
    /// plain slice; the test asserts both stay in sync).
    pub actions: &'static [ActionSpec],
    pub action_keys: &'static [&'static str],
}

pub(crate) fn invalid(message: String) -> String {
    rest_client::invalid(message)
}

/// Reads a non-empty trimmed string field from the credential envelope.
pub(crate) fn cred_str<'a>(
    credentials: &'a Value,
    field: &str,
) -> Result<&'a str, String> {
    credentials
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| invalid(format!("credential key {field} is missing or empty")))
}
