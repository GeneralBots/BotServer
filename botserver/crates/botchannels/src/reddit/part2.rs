use super::*;

impl RedditChannel {
    pub async fn submit_comment(&self, request: SubmitCommentRequest) -> Result<RedditComment, RedditError> {
        let token = self.get_access_token().await?;

        let response = self
            .http_client
            .post(format!("{}/api/comment", self.base_url))
            .bearer_auth(&token)
            .form(&[
                ("api_type", "json"),
                ("thing_id", &request.parent_id),
                ("text", &request.text),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let comment_response: CommentResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        if let Some(errors) = comment_response.json.errors {
            if !errors.is_empty() {
                let error_msg = errors
                    .iter()
                    .map(|e| e.join(": "))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(RedditError::ApiError(error_msg));
            }
        }

        let comment_data = comment_response
            .json
            .data
            .and_then(|d| d.things.into_iter().next())
            .and_then(|t| t.data)
            .ok_or_else(|| RedditError::ApiError("No comment data returned".to_string()))?;

        Ok(RedditComment {
            id: comment_data.id.unwrap_or_default(),
            body: comment_data.body.unwrap_or_default(),
            author: comment_data.author.unwrap_or_default(),
            score: comment_data.score.unwrap_or(0),
            created_utc: comment_data.created_utc.unwrap_or(0.0),
            parent_id: comment_data.parent_id.unwrap_or_default(),
            link_id: comment_data.link_id.unwrap_or_default(),
        })
    }

    pub async fn get_post(&self, post_id: &str) -> Result<RedditPost, RedditError> {
        let token = self.get_access_token().await?;

        let id = if post_id.starts_with("t3_") {
            post_id.to_string()
        } else {
            format!("t3_{}", post_id)
        };

        let response = self
            .http_client
            .get(format!("{}/api/info?id={}", self.base_url, id))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let listing: ListingResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let post_data = listing
            .data
            .children
            .into_iter()
            .next()
            .and_then(|c| c.data)
            .ok_or(RedditError::PostNotFound)?;

        Ok(RedditPost {
            id: post_data.id.unwrap_or_default(),
            subreddit: post_data.subreddit.unwrap_or_default(),
            title: post_data.title.unwrap_or_default(),
            selftext: post_data.selftext,
            url: post_data.url,
            author: post_data.author.unwrap_or_default(),
            score: post_data.score.unwrap_or(0),
            num_comments: post_data.num_comments.unwrap_or(0),
            created_utc: post_data.created_utc.unwrap_or(0.0),
            permalink: post_data.permalink.unwrap_or_default(),
            is_self: post_data.is_self.unwrap_or(false),
            link_flair_text: post_data.link_flair_text,
        })
    }

    pub async fn get_subreddit(&self, name: &str) -> Result<Subreddit, RedditError> {
        let token = self.get_access_token().await?;

        let response = self
            .http_client
            .get(format!("{}/r/{}/about", self.base_url, name))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let about: AboutResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let data = about.data;

        Ok(Subreddit {
            name: data.name.unwrap_or_default(),
            display_name: data.display_name.unwrap_or_default(),
            title: data.title.unwrap_or_default(),
            description: data.description,
            subscribers: data.subscribers.unwrap_or(0),
            public_description: data.public_description,
            subreddit_type: data.subreddit_type.unwrap_or_default(),
        })
    }

    pub async fn get_subreddit_posts(
        &self,
        subreddit: &str,
        sort: PostSort,
        limit: u32,
    ) -> Result<Vec<RedditPost>, RedditError> {
        let token = self.get_access_token().await?;

        let sort_str = match sort {
            PostSort::Hot => "hot",
            PostSort::New => "new",
            PostSort::Top => "top",
            PostSort::Rising => "rising",
            PostSort::Controversial => "controversial",
        };

        let response = self
            .http_client
            .get(format!(
                "{}/r/{}/{}?limit={}",
                self.base_url, subreddit, sort_str, limit
            ))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let listing: ListingResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let posts = listing
            .data
            .children
            .into_iter()
            .filter_map(|c| c.data)
            .map(|d| RedditPost {
                id: d.id.unwrap_or_default(),
                subreddit: d.subreddit.unwrap_or_default(),
                title: d.title.unwrap_or_default(),
                selftext: d.selftext,
                url: d.url,
                author: d.author.unwrap_or_default(),
                score: d.score.unwrap_or(0),
                num_comments: d.num_comments.unwrap_or(0),
                created_utc: d.created_utc.unwrap_or(0.0),
                permalink: d.permalink.unwrap_or_default(),
                is_self: d.is_self.unwrap_or(false),
                link_flair_text: d.link_flair_text,
            })
            .collect();

        Ok(posts)
    }

    pub async fn vote(&self, thing_id: &str, direction: VoteDirection) -> Result<(), RedditError> {
        let token = self.get_access_token().await?;

        let dir = match direction {
            VoteDirection::Up => "1",
            VoteDirection::Down => "-1",
            VoteDirection::None => "0",
        };

        let response = self
            .http_client
            .post(format!("{}/api/vote", self.base_url))
            .bearer_auth(&token)
            .form(&[("id", thing_id), ("dir", dir)])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        Ok(())
    }

    pub async fn delete(&self, thing_id: &str) -> Result<(), RedditError> {
        let token = self.get_access_token().await?;

        let response = self
            .http_client
            .post(format!("{}/api/del", self.base_url))
            .bearer_auth(&token)
            .form(&[("id", thing_id)])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        Ok(())
    }

    pub async fn edit(&self, thing_id: &str, new_text: &str) -> Result<(), RedditError> {
        let token = self.get_access_token().await?;

        let response = self
            .http_client
            .post(format!("{}/api/editusertext", self.base_url))
            .bearer_auth(&token)
            .form(&[
                ("api_type", "json"),
                ("thing_id", thing_id),
                ("text", new_text),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        Ok(())
    }

    pub async fn subscribe(&self, subreddit: &str, subscribe: bool) -> Result<(), RedditError> {
        let token = self.get_access_token().await?;

        let action = if subscribe { "sub" } else { "unsub" };

        let response = self
            .http_client
            .post(format!("{}/api/subscribe", self.base_url))
            .bearer_auth(&token)
            .form(&[
                ("action", action),
                ("sr_name", subreddit),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        Ok(())
    }

    pub async fn set_tokens(&self, tokens: RedditTokens) {
        let mut token_guard = self.tokens.write().await;
        *token_guard = Some(tokens);
    }
}
