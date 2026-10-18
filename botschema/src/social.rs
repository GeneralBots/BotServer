use crate::core::{branches};

diesel::table! {
    social_communities (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        cover_image -> Nullable<Text>,
        icon -> Nullable<Text>,
        visibility -> Varchar,
        join_policy -> Varchar,
        owner_id -> Uuid,
        member_count -> Int4,
        post_count -> Int4,
        is_official -> Bool,
        is_featured -> Bool,
        settings -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        archived_at -> Nullable<Timestamptz>,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    social_community_members (id) {
        id -> Uuid,
        community_id -> Uuid,
        user_id -> Uuid,
        role -> Varchar,
        notifications_enabled -> Bool,
        joined_at -> Timestamptz,
        last_seen_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_posts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        author_id -> Uuid,
        community_id -> Nullable<Uuid>,
        parent_id -> Nullable<Uuid>,
        content -> Text,
        content_type -> Varchar,
        attachments -> Jsonb,
        mentions -> Jsonb,
        hashtags -> Array<Nullable<Text>>,
        visibility -> Varchar,
        is_announcement -> Bool,
        is_pinned -> Bool,
        poll_id -> Nullable<Uuid>,
        reaction_counts -> Jsonb,
        comment_count -> Int4,
        share_count -> Int4,
        view_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    social_comments (id) {
        id -> Uuid,
        post_id -> Uuid,
        parent_comment_id -> Nullable<Uuid>,
        author_id -> Uuid,
        content -> Text,
        mentions -> Jsonb,
        reaction_counts -> Jsonb,
        reply_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        edited_at -> Nullable<Timestamptz>,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    social_reactions (id) {
        id -> Uuid,
        post_id -> Nullable<Uuid>,
        comment_id -> Nullable<Uuid>,
        user_id -> Uuid,
        reaction_type -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_polls (id) {
        id -> Uuid,
        post_id -> Uuid,
        question -> Text,
        allow_multiple -> Bool,
        allow_add_options -> Bool,
        anonymous -> Bool,
        total_votes -> Int4,
        ends_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_poll_options (id) {
        id -> Uuid,
        poll_id -> Uuid,
        text -> Varchar,
        vote_count -> Int4,
        position -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_poll_votes (id) {
        id -> Uuid,
        poll_id -> Uuid,
        option_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_announcements (id) {
        id -> Uuid,
        branch_id -> Uuid,
        author_id -> Uuid,
        title -> Varchar,
        content -> Text,
        priority -> Varchar,
        target_audience -> Jsonb,
        is_pinned -> Bool,
        requires_acknowledgment -> Bool,
        acknowledged_by -> Jsonb,
        starts_at -> Nullable<Timestamptz>,
        ends_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    social_praises (id) {
        id -> Uuid,
        branch_id -> Uuid,
        from_user_id -> Uuid,
        to_user_id -> Uuid,
        badge_type -> Varchar,
        message -> Nullable<Text>,
        is_public -> Bool,
        post_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    social_bookmarks (id) {
        id -> Uuid,
        user_id -> Uuid,
        post_id -> Uuid,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    social_hashtags (id) {
        id -> Uuid,
        branch_id -> Uuid,
        tag -> Varchar,
        post_count -> Int4,
        last_used_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(social_communities -> branches (branch_id));
diesel::joinable!(social_community_members -> social_communities (community_id));
diesel::joinable!(social_posts -> branches (branch_id));
diesel::joinable!(social_comments -> social_posts (post_id));
diesel::joinable!(social_polls -> social_posts (post_id));
diesel::joinable!(social_poll_options -> social_polls (poll_id));
diesel::joinable!(social_poll_votes -> social_polls (poll_id));
diesel::joinable!(social_poll_votes -> social_poll_options (option_id));
diesel::joinable!(social_announcements -> branches (branch_id));
diesel::joinable!(social_praises -> branches (branch_id));
diesel::joinable!(social_bookmarks -> social_posts (post_id));
diesel::joinable!(social_hashtags -> branches (branch_id));

diesel::table! {
    social_channel_accounts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        channel_type -> Varchar,
        credentials -> Jsonb,
        settings -> Jsonb,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(social_channel_accounts -> branches (branch_id));

diesel::allow_tables_to_appear_in_same_query!(
    social_communities,
    social_community_members,
    social_posts,
    social_comments,
    social_reactions,
    social_polls,
    social_poll_options,
    social_poll_votes,
    social_announcements,
    social_praises,
    social_bookmarks,
    social_hashtags,
    social_channel_accounts,
);
