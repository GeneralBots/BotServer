use super::*;

const BASECAMP_KEYS: &[&str] = &[
    "basecamp.work_items.list",
    "basecamp.todolists.list",
    "basecamp.work_items.create",
    "basecamp.todos.create",
];
pub const BASECAMP_SPEC: ProviderSpec = ProviderSpec {
    slug: "basecamp",
    origin: Origin::FromField {
        field: "base_url",
        pattern: "{value}",
    },
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: BASECAMP_ACTIONS,
    action_keys: BASECAMP_KEYS,
};
const LINKEDIN_ADS_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "linkedin_ads.campaigns.list",
    method: "GET",
    path: "/rest/adAccountGroups",
    summary: "Listed LinkedIn ad account groups.",
    path_params: &[],
    query: &[("pageSize", "limit")],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[s("limit")],
}];
const LINKEDIN_ADS_KEYS: &[&str] = &["linkedin_ads.campaigns.list"];
pub const LINKEDIN_ADS_SPEC: ProviderSpec = ProviderSpec {
    slug: "linkedin_ads",
    origin: Origin::Static("https://api.linkedin.com"),
    auth: AuthStyle::BearerHeaders {
        token_field: "token",
        headers: &[
            ("LinkedIn-Version", "202406"),
            ("X-Restli-Protocol", "2.0.0"),
        ],
    },
    actions: LINKEDIN_ADS_ACTIONS,
    action_keys: LINKEDIN_ADS_KEYS,
};
