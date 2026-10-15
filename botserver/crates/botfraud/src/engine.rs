use crate::rules;
use crate::scoring;
use crate::types::*;
use diesel::prelude::*;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

#[derive(Clone)]
pub struct FraudEngine {
    pub pool: DbPool,
}

/// The assessment outcome to persist as a fraud event.
struct FraudEventLog<'a> {
    conn: &'a mut PgConnection,
    branch: Uuid,
    request: &'a FraudAssessmentRequest,
    risk_score: i32,
    risk_level: &'a str,
    triggered: &'a [String],
    action_taken: &'a str,
}

impl FraudEngine {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn assess(&self, branch: Uuid, request: &FraudAssessmentRequest) -> FraudAssessmentResult {
        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(_) => {
                return FraudAssessmentResult {
                    risk_score: 0,
                    risk_level: "unknown".into(),
                    action_taken: "allow".into(),
                    triggered_rules: vec![],
                    ml_score: None,
                };
            }
        };

        let mut triggered = Vec::new();
        let mut total_score = 0i32;

        // 1. Load active rules scoped to the caller's branch
        let rules = rules::load_active_rules(&mut conn, branch);

        // 2. Evaluate rules
        for rule in &rules {
            if rules::evaluate_rule(rule, request) {
                let score = scoring::rule_score(&rule.severity);
                total_score += score;
                triggered.push(rule.name.clone());
            }
        }

        // 3. Check blocklist
        let blocked = self.check_blocklist(&mut conn, branch, request).unwrap_or(false);
        if blocked {
            total_score = 100;
            triggered.push("blocklist_match".into());
        }

        // 4. Check velocity
        let velo = self.check_velocity(&mut conn, branch, request).unwrap_or(false);
        if velo {
            total_score += 30;
            triggered.push("velocity_threshold".into());
        }

        // 5. Clamp score and determine level
        let risk_score = total_score.clamp(0, 100);
        let (risk_level, action_taken) = scoring::classify(risk_score, blocked);

        // 6. Log event
        let _ = Self::log_event(FraudEventLog {
            conn: &mut conn,
            branch,
            request,
            risk_score,
            risk_level: &risk_level,
            triggered: &triggered,
            action_taken: &action_taken,
        });

        FraudAssessmentResult {
            risk_score,
            risk_level,
            action_taken,
            triggered_rules: triggered,
            ml_score: None,
        }
    }

    fn check_blocklist(
        &self,
        conn: &mut PgConnection,
        branch: Uuid,
        request: &FraudAssessmentRequest,
    ) -> Result<bool, diesel::result::Error> {
        let details = &request.details;
        let checks = ["ip", "email", "phone", "cpf", "cnpj", "card_bin"];

        for check in &checks {
            if let Some(val) = details.get(check).and_then(|v| v.as_str()) {
                let exists = diesel::sql_query(
                    "SELECT COUNT(*) as cnt FROM fraud_blocklist \
                     WHERE branch_id = $1 AND block_type = $2 AND block_value = $3 \
                     AND (expires_at IS NULL OR expires_at > NOW())",
                )
                .bind::<diesel::sql_types::Uuid, _>(branch)
                .bind::<diesel::sql_types::Text, _>(check)
                .bind::<diesel::sql_types::Text, _>(val)
                .get_result::<BlocklistCount>(conn)
                .map(|r| r.cnt > 0)
                .unwrap_or(false);

                if exists {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn check_velocity(
        &self,
        conn: &mut PgConnection,
        branch: Uuid,
        request: &FraudAssessmentRequest,
    ) -> Result<bool, diesel::result::Error> {
        let ip = request.details.get("ip").and_then(|v| v.as_str());
        let email = request.details.get("email").and_then(|v| v.as_str());

        let checks = [("ip", ip), ("email", email)];

        for (id_type, id_val) in &checks {
            if let Some(val) = id_val {
                let result = diesel::sql_query(
                    "SELECT COUNT(*) as cnt FROM fraud_velocity \
                     WHERE branch_id = $1 AND identifier_type = $2 AND identifier = $3 \
                     AND event_type = $4 \
                     AND window_start > NOW() - INTERVAL '1 hour'",
                )
                .bind::<diesel::sql_types::Uuid, _>(branch)
                .bind::<diesel::sql_types::Text, _>(id_type)
                .bind::<diesel::sql_types::Text, _>(val)
                .bind::<diesel::sql_types::Text, _>(&request.event_type)
                .get_result::<BlocklistCount>(conn)
                .map(|r| r.cnt > 10)
                .unwrap_or(false);

                if result {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// The assessment outcome to persist as a fraud event.
    fn log_event(event: FraudEventLog<'_>) -> Result<(), diesel::result::Error> {
        let FraudEventLog {
            conn,
            branch,
            request,
            risk_score,
            risk_level,
            triggered,
            action_taken,
        } = event;
        let id = Uuid::new_v4();
        let rules_json: serde_json::Value =
            serde_json::to_value(triggered).unwrap_or_default();
        diesel::sql_query(
            "INSERT INTO fraud_events (id, branch_id, event_type, entity_type, entity_id, \
             risk_score, risk_level, triggered_rules, action_taken, details) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind::<diesel::sql_types::Uuid, _>(&id)
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .bind::<diesel::sql_types::Text, _>(&request.event_type)
        .bind::<diesel::sql_types::Text, _>(&request.entity_type)
        .bind::<diesel::sql_types::Uuid, _>(&request.entity_id)
        .bind::<diesel::sql_types::Integer, _>(&risk_score)
        .bind::<diesel::sql_types::Text, _>(risk_level)
        .bind::<diesel::sql_types::Jsonb, _>(&rules_json)
        .bind::<diesel::sql_types::Text, _>(action_taken)
        .bind::<diesel::sql_types::Jsonb, _>(&request.details)
        .execute(conn)?;

        // Update velocity counter
        if let Some(ip) = request.details.get("ip").and_then(|v| v.as_str()) {
            diesel::sql_query(
                "INSERT INTO fraud_velocity (id, branch_id, identifier, identifier_type, event_type) \
                 VALUES ($1, $2, $3, 'ip', $4)",
            )
            .bind::<diesel::sql_types::Uuid, _>(&Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .bind::<diesel::sql_types::Text, _>(ip)
            .bind::<diesel::sql_types::Text, _>(&request.event_type)
            .execute(conn)?;
        }

        Ok(())
    }
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct BlocklistCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    cnt: i64,
}
