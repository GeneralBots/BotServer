use super::*;

#[test]
pub(super) fn test_engagement_to_dynamic() {
    let engagement = PostEngagement {
        likes: 100,
        comments: 20,
        shares: 5,
        views: 1000,
        clicks: 50,
        reach: 500,
    };

    let dynamic = engagement.to_dynamic();
    assert!(dynamic.is_map());
}
