use crate::sync_types::{ExternalContact, ExternalEmail, ExternalPhone};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct GoogleClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
}

pub struct GoogleContactsClient {
    config: GoogleConfig,
    client: Client,
}

impl GoogleContactsClient {
    pub fn new(config: GoogleConfig) -> Self {
        Self { config, client: Client::new() }
    }

    pub fn get_auth_url(&self, redirect_uri: &str, state: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&redirect_uri={}&response_type=code&scope=https://www.googleapis.com/auth/contacts&state={}",
            self.config.client_id, redirect_uri, state
        )
    }

    pub async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<TokenResponse, GoogleError> {
        let response = self.client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError(format!("Token exchange failed: {}", response.status())));
        }

        #[derive(Deserialize)]
        struct GoogleTokenResponse { access_token: String, refresh_token: Option<String>, expires_in: i64, scope: Option<String> }

        let token_data: GoogleTokenResponse = response.json().await
            .map_err(|e| GoogleError::ParseError(e.to_string()))?;

        Ok(TokenResponse {
            access_token: token_data.access_token,
            refresh_token: token_data.refresh_token,
            expires_in: token_data.expires_in,
            expires_at: Some(Utc::now() + chrono::Duration::seconds(token_data.expires_in)),
            scopes: token_data.scope.map(|s| s.split(' ').map(String::from).collect()).unwrap_or_default(),
        })
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<UserInfo, GoogleError> {
        let response = self.client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .bearer_auth(access_token)
            .send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError("Failed to get user info".to_string()));
        }

        #[derive(Deserialize)]
        struct GoogleUserInfo { id: String, email: String, name: Option<String> }

        let info: GoogleUserInfo = response.json().await
            .map_err(|e| GoogleError::ParseError(e.to_string()))?;

        Ok(UserInfo { id: info.id, email: info.email, name: info.name })
    }

    pub async fn revoke_token(&self, _access_token: &str) -> Result<(), GoogleError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub scopes: Vec<String>,
}

impl GoogleClient {
    pub fn new() -> Self {
        Self { client: Client::new(), base_url: "https://people.googleapis.com/v1".to_string() }
    }

    pub async fn fetch_contacts(&self, access_token: &str) -> Result<(Vec<ExternalContact>, Option<String>), GoogleError> {
        let mut all_contacts = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let (contacts, next_token) = self.list_contacts(access_token, page_token.as_deref()).await?;
            all_contacts.extend(contacts);
            if next_token.is_none() { break; }
            page_token = next_token;
            if all_contacts.len() > 10000 { break; }
        }

        Ok((all_contacts, None))
    }

    pub async fn list_contacts(&self, access_token: &str, page_token: Option<&str>) -> Result<(Vec<ExternalContact>, Option<String>), GoogleError> {
        let mut url = format!("{}/people/me/connections?personFields=names,emailAddresses,phoneNumbers,organizations,biographies", self.base_url);
        if let Some(token) = page_token { url.push_str(&format!("&pageToken={}", token)); }

        let response = self.client.get(&url).bearer_auth(access_token).send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError(format!("Failed to list contacts: {}", response.status())));
        }

        #[derive(Deserialize)]
        struct GoogleResponse { connections: Option<Vec<GooglePerson>>, next_page_token: Option<String> }

        let data: GoogleResponse = response.json().await
            .map_err(|e| GoogleError::ParseError(e.to_string()))?;

        let contacts = data.connections.unwrap_or_default().into_iter().map(|person| {
        let first_name = person.names.as_ref().and_then(|n| n.first().map(|n| n.given_name.clone()));
        let last_name = person.names.as_ref().and_then(|n| n.first().map(|n| n.family_name.clone()));
        let display_name = person.names.as_ref().and_then(|n| n.first().and_then(|n| n.display_name.clone()));

        let email = person.email_addresses.as_ref().and_then(|emails| {
            emails.first().map(|em| ExternalEmail {
                address: em.value.clone(),
                label: None,
                primary: em.metadata.as_ref().and_then(|m| m.primary).unwrap_or(false),
            })
        });

        let phone = person.phone_numbers.as_ref().and_then(|phones| {
            phones.first().map(|p| ExternalPhone {
                number: p.value.clone().unwrap_or_default(),
                label: None,
                primary: p.metadata.as_ref().and_then(|m| m.primary).unwrap_or(false),
            })
        });

            ExternalContact {
                id: person.resource_name.unwrap_or_default(), etag: person.etag,
                first_name, last_name, display_name,
                email_addresses: email.map(|e| vec![e]).unwrap_or_default(),
                phone_numbers: phone.map(|p| vec![p]).unwrap_or_default(),
                addresses: vec![],
                company: person.organizations.as_ref().and_then(|o| o.first().and_then(|org| org.name.clone())),
                job_title: person.organizations.as_ref().and_then(|o| o.first().and_then(|org| org.title.clone())),
                department: None,
                notes: person.biographies.as_ref().and_then(|b| b.first().and_then(|bio| bio.content.clone())),
                birthday: None, photo_url: person.photos.as_ref().and_then(|photos| photos.first().and_then(|photo| photo.url.clone())),
                groups: vec![], custom_fields: Default::default(), created_at: None, updated_at: None,
            }
        }).collect();

        Ok((contacts, data.next_page_token))
    }

    pub async fn create_contact(&self, access_token: &str, contact: &ExternalContact) -> Result<String, GoogleError> {
        let body = serde_json::json!({
            "names": [{ "givenName": contact.first_name, "familyName": contact.last_name, "displayName": contact.display_name }],
            "emailAddresses": if contact.email_addresses.is_empty() { None } else {
                Some(contact.email_addresses.iter().map(|e| serde_json::json!({ "value": e.address, "metadata": {"primary": e.primary} })).collect::<Vec<_>>())
            },
            "phoneNumbers": if contact.phone_numbers.is_empty() { None } else {
                Some(contact.phone_numbers.iter().map(|p| serde_json::json!({ "value": p.number, "metadata": {"primary": p.primary} })).collect::<Vec<_>>())
            },
            "organizations": if contact.company.is_some() || contact.job_title.is_some() {
                Some(vec![serde_json::json!({ "name": contact.company.as_ref().unwrap_or(&String::new()), "title": contact.job_title.as_ref().unwrap_or(&String::new()) })])
            } else { None }
        });

        let response = self.client
            .post(format!("{}/people/me/connections:create", self.base_url))
            .query(&[("personFields", "names,emailAddresses,phoneNumbers,organizations")])
            .bearer_auth(access_token).json(&body).send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError(format!("Create contact failed: {}", response.status())));
        }

        #[derive(Deserialize)]
        struct CreateResponse { resource_name: String }
        let data: CreateResponse = response.json().await.map_err(|e| GoogleError::ParseError(e.to_string()))?;
        Ok(data.resource_name)
    }

    pub async fn update_contact(&self, access_token: &str, resource_name: &str, contact: &ExternalContact) -> Result<(), GoogleError> {
        let body = serde_json::json!({
            "names": [{ "givenName": contact.first_name, "familyName": contact.last_name, "displayName": contact.display_name }],
            "emailAddresses": if contact.email_addresses.is_empty() { None } else {
                Some(contact.email_addresses.iter().map(|e| serde_json::json!({ "value": e.address, "metadata": {"primary": e.primary} })).collect::<Vec<_>>())
            },
            "phoneNumbers": if contact.phone_numbers.is_empty() { None } else {
                Some(contact.phone_numbers.iter().map(|p| serde_json::json!({ "value": p.number, "metadata": {"primary": p.primary} })).collect::<Vec<_>>())
            },
            "organizations": if contact.company.is_some() || contact.job_title.is_some() {
                Some(vec![serde_json::json!({ "name": contact.company.as_ref().unwrap_or(&String::new()), "title": contact.job_title.as_ref().unwrap_or(&String::new()) })])
            } else { None }
        });

        let response = self.client
            .patch(format!("{}/people/me/{}:update", self.base_url, resource_name))
            .query(&[("personFields", "names,emailAddresses,phoneNumbers,organizations")])
            .bearer_auth(access_token).json(&body).send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError(format!("Update contact failed: {}", response.status())));
        }
        Ok(())
    }

    pub async fn delete_contact(&self, access_token: &str, resource_name: &str) -> Result<(), GoogleError> {
        let response = self.client
            .delete(format!("{}/people/me/{}", self.base_url, resource_name))
            .bearer_auth(access_token).send().await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(GoogleError::ApiError(format!("Delete contact failed: {}", response.status())));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum GoogleError { NetworkError(String), ApiError(String), ParseError(String) }

impl std::fmt::Display for GoogleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(e) => write!(f, "Network error: {e}"),
            Self::ApiError(e) => write!(f, "API error: {e}"),
            Self::ParseError(e) => write!(f, "Parse error: {e}"),
        }
    }
}

impl std::error::Error for GoogleError {}

impl From<GoogleError> for crate::sync_types::ExternalSyncError {
    fn from(e: GoogleError) -> Self {
        crate::sync_types::ExternalSyncError {
            kind: match &e {
                GoogleError::NetworkError(_) => crate::sync_types::ExternalSyncErrorKind::NetworkError,
                GoogleError::ApiError(_) => crate::sync_types::ExternalSyncErrorKind::ApiError,
                GoogleError::ParseError(_) => crate::sync_types::ExternalSyncErrorKind::ParseError,
            },
            message: e.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GooglePerson {
    resource_name: Option<String>, etag: Option<String>,
    email_addresses: Option<Vec<GoogleEmail>>,
    phone_numbers: Option<Vec<GooglePhone>>, organizations: Option<Vec<GoogleOrganization>>,
    biographies: Option<Vec<GoogleBiography>>, photos: Option<Vec<GooglePhoto>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleEmail { value: String, metadata: Option<GoogleMetadata> }

#[derive(Debug, Clone, Deserialize)]
struct GooglePhone { value: Option<String>, metadata: Option<GoogleMetadata> }

#[derive(Debug, Clone, Deserialize)]
struct GoogleOrganization { name: Option<String>, title: Option<String> }

#[derive(Debug, Clone, Deserialize)]
struct GoogleBiography { content: Option<String> }

#[derive(Debug, Clone, Deserialize)]
struct GooglePhoto { url: Option<String> }

#[derive(Debug, Clone, Deserialize)]
struct GoogleMetadata { primary: Option<bool> }
