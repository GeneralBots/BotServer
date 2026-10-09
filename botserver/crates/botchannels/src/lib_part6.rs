use super::*;

impl MultiPostResult {
    pub fn from_results(results: Vec<Result<PostResult, ChannelError>>) -> Self {
        let total = results.len();
        let mut successful = 0;
        let mut failed = 0;
        let mut post_results = Vec::with_capacity(total);

        for result in results {
            match result {
                Ok(r) => {
                    if r.success {
                        successful += 1;
                    } else {
                        failed += 1;
                    }
                    post_results.push(r);
                }
                Err(e) => {
                    failed += 1;
                    post_results.push(PostResult {
                        success: false,
                        channel_type: ChannelType::Twitter,
                        post_id: None,
                        url: None,
                        error: Some(e.to_string()),
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Self {
            total,
            successful,
            failed,
            results: post_results,
        }
    }
}
