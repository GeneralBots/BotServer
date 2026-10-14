use super::*;

pub(crate) fn record_to_objective(record: ObjectiveRecord) -> Objective {
    Objective {
        id: record.id,
        organization_id: record.org_id,
        owner_id: record.owner_id,
        owner_name: None,
        parent_id: record.parent_id,
        title: record.title,
        description: record.description.unwrap_or_default(),
        period: record.period,
        period_start: record.period_start,
        period_end: record.period_end,
        status: ObjectiveStatus::from_label(&record.status),
        progress: record.progress.to_f32().unwrap_or(0.0),
        visibility: Visibility::from_label(&record.visibility),
        weight: record.weight.to_f32().unwrap_or(1.0),
        tags: record.tags,
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
pub(crate) fn record_to_key_result(record: KeyResultRecord) -> KeyResult {
    KeyResult {
        id: record.id,
        objective_id: record.objective_id,
        owner_id: record.owner_id,
        title: record.title,
        description: record.description,
        metric_type: MetricType::from_label(&record.metric_type),
        start_value: record.start_value.to_f64().unwrap_or(0.0),
        target_value: record.target_value.to_f64().unwrap_or(0.0),
        current_value: record.current_value.to_f64().unwrap_or(0.0),
        unit: record.unit,
        weight: record.weight.to_f32().unwrap_or(1.0),
        due_date: record.due_date,
        status: KRStatus::from_label(&record.status),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}
pub(crate) fn record_to_checkin(record: CheckInRecord) -> CheckIn {
    CheckIn {
        id: record.id,
        key_result_id: record.key_result_id,
        user_id: record.user_id,
        previous_value: record.previous_value.and_then(|v| v.to_f64()).unwrap_or(0.0),
        new_value: record.new_value.to_f64().unwrap_or(0.0),
        note: record.note.unwrap_or_default(),
        confidence: Confidence::from_label(record.confidence.as_deref().unwrap_or("medium")),
        blockers: record.blockers,
        created_at: record.created_at,
    }
}
