use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::*;

#[derive(Clone)]
pub struct GamificationService {
    badges: Vec<BadgeDefinition>,
    achievements: Vec<Achievement>,
    xp_ledger: Vec<XpTransaction>,
    leaderboard_cache: Vec<LeaderboardEntry>,
}

impl GamificationService {
    pub fn new() -> Self {
        let badges = Self::default_badges();
        Self {
            badges,
            achievements: Vec::new(),
            xp_ledger: Vec::new(),
            leaderboard_cache: Vec::new(),
        }
    }

    fn default_badges() -> Vec<BadgeDefinition> {
        vec![
            BadgeDefinition {
                badge_type: "first_course".to_string(),
                name: "First Steps".to_string(),
                description: "Complete your first course".to_string(),
                icon_url: None,
                xp_reward: 100,
                criteria: BadgeCriteria {
                    action: "course_complete".to_string(),
                    threshold: 1,
                    scope: "global".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "five_courses".to_string(),
                name: "Knowledge Seeker".to_string(),
                description: "Complete 5 courses".to_string(),
                icon_url: None,
                xp_reward: 500,
                criteria: BadgeCriteria {
                    action: "course_complete".to_string(),
                    threshold: 5,
                    scope: "global".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "perfect_score".to_string(),
                name: "Perfect Score".to_string(),
                description: "Achieve 100% on any quiz".to_string(),
                icon_url: None,
                xp_reward: 250,
                criteria: BadgeCriteria {
                    action: "quiz_perfect".to_string(),
                    threshold: 1,
                    scope: "global".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "streak_7".to_string(),
                name: "Weekly Warrior".to_string(),
                description: "Maintain a 7-day learning streak".to_string(),
                icon_url: None,
                xp_reward: 300,
                criteria: BadgeCriteria {
                    action: "login_streak".to_string(),
                    threshold: 7,
                    scope: "global".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "ten_courses".to_string(),
                name: "Course Collector".to_string(),
                description: "Complete 10 courses".to_string(),
                icon_url: None,
                xp_reward: 1000,
                criteria: BadgeCriteria {
                    action: "course_complete".to_string(),
                    threshold: 10,
                    scope: "global".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "speed_demon".to_string(),
                name: "Speed Demon".to_string(),
                description: "Complete a course in under 1 hour".to_string(),
                icon_url: None,
                xp_reward: 200,
                criteria: BadgeCriteria {
                    action: "course_quick".to_string(),
                    threshold: 60,
                    scope: "minutes".to_string(),
                },
            },
            BadgeDefinition {
                badge_type: "all_quizzes".to_string(),
                name: "Quiz Master".to_string(),
                description: "Pass all quizzes in a course with 80%+".to_string(),
                icon_url: None,
                xp_reward: 400,
                criteria: BadgeCriteria {
                    action: "all_quizzes_passed".to_string(),
                    threshold: 80,
                    scope: "percentage".to_string(),
                },
            },
        ]
    }

    pub fn award_badge(
        &mut self,
        user_id: Uuid,
        badge_type: &str,
    ) -> Option<AchievementResponse> {
        let badge_def = self.badges.iter().find(|b| b.badge_type == badge_type)?;

        let achievement = Achievement {
            id: Uuid::new_v4(),
            user_id,
            badge_type: badge_type.to_string(),
            earned_at: Utc::now(),
            criteria_met: serde_json::json!({}),
            badge_name: Some(badge_def.name.clone()),
            badge_description: Some(badge_def.description.clone()),
            badge_icon_url: badge_def.icon_url.clone(),
        };

        self.add_xp(user_id, badge_def.xp_reward, &format!("Badge: {}", badge_def.name));

        let response = AchievementResponse {
            id: achievement.id,
            badge_type: achievement.badge_type.clone(),
            badge_name: achievement.badge_name.clone(),
            badge_description: achievement.badge_description.clone(),
            badge_icon_url: achievement.badge_icon_url.clone(),
            earned_at: achievement.earned_at,
        };

        self.achievements.push(achievement);
        Some(response)
    }

    pub fn add_xp(
        &mut self,
        user_id: Uuid,
        amount: i32,
        reason: &str,
    ) -> XpTransaction {
        let tx = XpTransaction {
            id: Uuid::new_v4(),
            user_id,
            amount,
            reason: reason.to_string(),
            reference_type: None,
            reference_id: None,
            created_at: Utc::now(),
        };
        self.xp_ledger.push(tx.clone());
        self.invalidate_leaderboard();
        tx
    }

    pub fn get_user_xp(&self, user_id: Uuid) -> i32 {
        self.xp_ledger
            .iter()
            .filter(|tx| tx.user_id == user_id)
            .map(|tx| tx.amount)
            .sum()
    }

    pub fn get_user_level(&self, user_id: Uuid) -> UserLevelInfo {
        let total_xp = self.get_user_xp(user_id);
        let mut info = xp_progress(total_xp);
        info.user_id = user_id;
        info
    }

    pub fn get_user_achievements(&self, user_id: Uuid) -> Vec<AchievementResponse> {
        self.achievements
            .iter()
            .filter(|a| a.user_id == user_id)
            .map(|a| AchievementResponse {
                id: a.id,
                badge_type: a.badge_type.clone(),
                badge_name: a.badge_name.clone(),
                badge_description: a.badge_description.clone(),
                badge_icon_url: a.badge_icon_url.clone(),
                earned_at: a.earned_at,
            })
            .collect()
    }

    pub fn get_leaderboard(&mut self, limit: usize) -> Vec<LeaderboardEntry> {
        if !self.leaderboard_cache.is_empty() {
            return self.leaderboard_cache.iter().take(limit).cloned().collect();
        }

        let mut xp_by_user: HashMap<Uuid, i32> = HashMap::new();
        for tx in &self.xp_ledger {
            *xp_by_user.entry(tx.user_id).or_insert(0) += tx.amount;
        }

        let mut entries: Vec<(Uuid, i32)> = xp_by_user.into_iter().collect();
        entries.sort_by_key(|a| std::cmp::Reverse(a.1.clone()));

        let leaderboard: Vec<LeaderboardEntry> = entries
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(i, (user_id, total_xp))| {
                let achievement_count = self.achievements.iter()
                    .filter(|a| a.user_id == user_id)
                    .count() as i32;
                let completed = self.achievements.iter()
                    .filter(|a| a.user_id == user_id && a.badge_type == "first_course")
                    .count() as i32;

                LeaderboardEntry {
                    user_id,
                    user_name: format!("User {}", user_id.to_string().split('-').next().unwrap_or("0")),
                    avatar_url: None,
                    total_xp,
                    level: calculate_level(total_xp),
                    badges_count: achievement_count,
                    courses_completed: completed,
                    rank: (i + 1) as i32,
                }
            })
            .collect();

        self.leaderboard_cache = leaderboard.clone();
        leaderboard
    }

    pub fn get_badge_definitions(&self) -> &[BadgeDefinition] {
        &self.badges
    }

    fn invalidate_leaderboard(&mut self) {
        self.leaderboard_cache.clear();
    }

    pub fn check_and_award(&mut self, user_id: Uuid, action: &str, value: i32) -> Vec<AchievementResponse> {
        let to_award: Vec<String> = self.badges.iter()
            .filter(|badge| badge.criteria.action == action && value >= badge.criteria.threshold)
            .filter(|badge| !self.achievements.iter().any(|a| a.user_id == user_id && a.badge_type == badge.badge_type))
            .map(|badge| badge.badge_type.clone())
            .collect();

        let mut awarded = Vec::new();
        for badge_type in to_award {
            if let Some(ach) = self.award_badge(user_id, &badge_type) {
                awarded.push(ach);
            }
        }
        awarded
    }
}

impl Default for GamificationService {
    fn default() -> Self {
        Self::new()
    }
}
