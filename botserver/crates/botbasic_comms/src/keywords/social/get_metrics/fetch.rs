use super::*;

pub(super) async fn fetch_instagram_metrics(
    state: &dyn BasicRuntime,
    user: &UserSession,
    post_id: &str,
) -> Result<PostEngagement, String> {
    let credentials = get_platform_credentials(state, user.bot_id, "instagram")?;

    let access_token = credentials
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token")?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://graph.facebook.com/v18.0/{}", post_id))
        .query(&[
            ("fields", "like_count,comments_count,impressions_count"),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| format!("Instagram API error: {}", e))?;

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(PostEngagement {
        likes: data.get("like_count").and_then(|v| v.as_i64()).unwrap_or(0),
        comments: data
            .get("comments_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        views: data
            .get("impressions_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        ..Default::default()
    })
}

pub(super) async fn fetch_facebook_metrics(
    state: &dyn BasicRuntime,
    user: &UserSession,
    post_id: &str,
) -> Result<PostEngagement, String> {
    let credentials = get_platform_credentials(state, user.bot_id, "facebook")?;

    let access_token = credentials
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token")?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://graph.facebook.com/v18.0/{}", post_id))
        .query(&[
            (
                "fields",
                "likes.summary(true),comments.summary(true),shares",
            ),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| format!("Facebook API error: {}", e))?;

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(PostEngagement {
        likes: data
            .get("likes")
            .and_then(|l| l.get("summary"))
            .and_then(|s| s.get("total_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        comments: data
            .get("comments")
            .and_then(|c| c.get("summary"))
            .and_then(|s| s.get("total_count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        shares: data
            .get("shares")
            .and_then(|s| s.get("count"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        ..Default::default()
    })
}

pub(super) async fn fetch_linkedin_metrics(
    state: &dyn BasicRuntime,
    user: &UserSession,
    post_id: &str,
) -> Result<PostEngagement, String> {
    let credentials = get_platform_credentials(state, user.bot_id, "linkedin")?;

    let access_token = credentials
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token")?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "https://api.linkedin.com/v2/socialActions/{}/summary",
            post_id
        ))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("LinkedIn API error: {}", e))?;

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    Ok(PostEngagement {
        likes: data
            .get("likesSummary")
            .and_then(|l| l.get("totalLikes"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        comments: data
            .get("commentsSummary")
            .and_then(|c| c.get("totalFirstLevelComments"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        ..Default::default()
    })
}

pub(super) async fn fetch_twitter_metrics(
    state: &dyn BasicRuntime,
    user: &UserSession,
    post_id: &str,
) -> Result<PostEngagement, String> {
    let credentials = get_platform_credentials(state, user.bot_id, "twitter")?;

    let bearer_token = credentials
        .get("bearer_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing bearer_token")?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("https://api.twitter.com/2/tweets/{}", post_id))
        .query(&[("tweet.fields", "public_metrics")])
        .header("Authorization", format!("Bearer {}", bearer_token))
        .send()
        .await
        .map_err(|e| format!("Twitter API error: {}", e))?;

    let data: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let metrics = data
        .get("data")
        .and_then(|d| d.get("public_metrics"))
        .cloned()
        .unwrap_or_default();

    Ok(PostEngagement {
        likes: metrics
            .get("like_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        comments: metrics
            .get("reply_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        shares: metrics
            .get("retweet_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        views: metrics
            .get("impression_count")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        ..Default::default()
    })
}
