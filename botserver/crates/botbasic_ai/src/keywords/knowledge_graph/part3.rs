use super::*;

pub fn register_knowledge_graph_keywords(engine: &mut Engine) {
    engine.register_fn("entity_name", |entity: Map| -> String {
        entity
            .get("entity_name")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    engine.register_fn("entity_type", |entity: Map| -> String {
        entity
            .get("entity_type")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    engine.register_fn("entity_properties", |entity: Map| -> Map {
        entity
            .get("properties")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default()
    });

    engine.register_fn("relationship_type", |rel: Map| -> String {
        rel.get("relationship_type")
            .and_then(|v| v.clone().try_cast::<String>())
            .unwrap_or_default()
    });

    engine.register_fn("is_bidirectional", |rel: Map| -> bool {
        rel.get("bidirectional")
            .and_then(|v| v.clone().try_cast::<bool>())
            .unwrap_or(false)
    });

    info!("Knowledge graph keywords registered");
}
