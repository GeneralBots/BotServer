use super::*;

impl Default for DataImpact {
    fn default() -> Self {
        Self {
            records_created: 0,
            records_modified: 0,
            records_deleted: 0,
            tables_affected: Vec::new(),
            data_sources_affected: Vec::new(),
            reversible: true,
            backup_required: false,
        }
    }
}
