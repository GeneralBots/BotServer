use super::*;

impl SynchronizeService {
    pub(super) fn transform_record(
        &self,
        record: &Value,
        config: &SynchronizeConfig,
    ) -> Result<Value, SynchronizeError> {
        let Some(transform) = &config.transform else {
            return Ok(record.clone());
        };

        let Some(obj) = record.as_object() else {
            return Ok(record.clone());
        };

        let mut result: Map<String, Value> = Map::new();

        for (key, value) in obj {
            if let Some(exclude) = &transform.exclude_fields {
                if exclude.contains(key) {
                    continue;
                }
            }

            if let Some(include) = &transform.include_fields {
                if !include.contains(key) {
                    continue;
                }
            }

            let target_key = if let Some(mappings) = &transform.field_mappings {
                mappings.get(key).cloned().unwrap_or_else(|| key.clone())
            } else {
                key.clone()
            };

            let coerced_value = if let Some(coercions) = &transform.type_coercions {
                if let Some(target_type) = coercions.get(&target_key) {
                    coerce_value(value, target_type)?
                } else {
                    value.clone()
                }
            } else {
                value.clone()
            };

            result.insert(target_key, coerced_value);
        }

        if let Some(computed) = &transform.computed_fields {
            for field in computed {
                let computed_value = self.compute_field(record, field)?;
                result.insert(field.name.clone(), computed_value);
            }
        }

        Ok(Value::Object(result))
    }

    pub(super) fn compute_field(
        &self,
        _record: &Value,
        field: &ComputedField,
    ) -> Result<Value, SynchronizeError> {
        match field.expression.as_str() {
            "NOW()" | "now()" => Ok(Value::String(Utc::now().to_rfc3339())),
            "UUID()" | "uuid()" => Ok(Value::String(Uuid::new_v4().to_string())),
            expr if expr.starts_with("CONCAT(") => {
                Ok(Value::String(format!("computed:{}", field.name)))
            }
            _ => Ok(Value::Null),
        }
    }

    async fn find_existing_record(
        &self,
        _bot_id: Uuid,
        _table_name: &str,
        _key_field: &str,
        _key_value: &str,
    ) -> Result<Option<Value>, SynchronizeError> {
        Ok(None)
    }

    async fn insert_record(
        &self,
        _bot_id: Uuid,
        _table_name: &str,
        _record: &Value,
    ) -> Result<(), SynchronizeError> {
        Ok(())
    }

    async fn update_record(
        &self,
        _bot_id: Uuid,
        _table_name: &str,
        _key_field: &str,
        _key_value: &str,
        _record: &Value,
    ) -> Result<(), SynchronizeError> {
        Ok(())
    }

    async fn handle_retry(&self, _page: u32) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_millis(RETRY_DELAY_MS))
    }
}
