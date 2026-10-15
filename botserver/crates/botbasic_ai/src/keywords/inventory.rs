use botbasic_types::BasicRuntime;
use botinventory::InventoryRequest;
use botbasic_types::UserSession;
use log::trace;
use rhai::{Dynamic, Engine};

use std::sync::Arc;

/// Inventory and POS BASIC keywords for issue #620.
///
/// Provides: CREATE PRODUCT, ADD VARIATION, SET STOCK, TRANSFER STOCK,
/// GET STOCK, SET PRICE, OPEN POS SESSION, ADD TO CART, CHECKOUT.
pub fn register_inventory_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_create_product(state.clone(), user.clone(), engine);
    register_add_variation(state.clone(), user.clone(), engine);
    register_set_stock(state.clone(), user.clone(), engine);
    register_transfer_stock(state.clone(), user.clone(), engine);
    register_get_stock(state.clone(), user.clone(), engine);
    register_set_price(state.clone(), user.clone(), engine);
    register_open_pos_session(state.clone(), user.clone(), engine);
    register_add_to_cart(state.clone(), user.clone(), engine);
    register_checkout(state, user, engine);
}

fn register_create_product(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["CREATE", "PRODUCT", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let name = context.eval_expression_tree(&inputs[0])?.to_string();
                let sku = context.eval_expression_tree(&inputs[1])?.to_string();
                let price_expr = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("CREATE PRODUCT: {name} sku={sku} price={price_expr}");
                let request = InventoryRequest::create_product(&name, &sku, &price_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("CREATE PRODUCT syntax failed: {e}");
    }
}

fn register_add_variation(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "ADD", "VARIATION", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let product_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let sku = context.eval_expression_tree(&inputs[1])?.to_string();
                let name = context.eval_expression_tree(&inputs[2])?.to_string();
                let price_expr = context.eval_expression_tree(&inputs[3])?.to_string();
                trace!("ADD VARIATION: {name} sku={sku} to {product_id}");
                let request =
                    InventoryRequest::add_variation(&product_id, &sku, &name, &price_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("ADD VARIATION syntax failed: {e}");
    }
}

fn register_set_stock(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "STOCK", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let product_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let branch_id = context.eval_expression_tree(&inputs[1])?.to_string();
                let qty_expr = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("SET STOCK: product={product_id} branch={branch_id} qty={qty_expr}");
                let request = InventoryRequest::set_stock(&product_id, &branch_id, &qty_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("SET STOCK syntax failed: {e}");
    }
}

fn register_transfer_stock(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "TRANSFER", "STOCK", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let product_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let from_branch = context.eval_expression_tree(&inputs[1])?.to_string();
                let to_branch = context.eval_expression_tree(&inputs[2])?.to_string();
                let qty_expr = context.eval_expression_tree(&inputs[3])?.to_string();
                trace!("TRANSFER STOCK: {product_id} {from_branch}->{to_branch} qty={qty_expr}");
                let request = InventoryRequest::transfer_stock(
                    &product_id,
                    &from_branch,
                    &to_branch,
                    &qty_expr,
                );
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("TRANSFER STOCK syntax failed: {e}");
    }
}

fn register_get_stock(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "STOCK", "$expr$"],
            false,
            move |context, inputs| {
                let product_id = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("GET STOCK: {product_id}");
                let request = InventoryRequest::get_stock(&product_id);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("GET STOCK syntax failed: {e}");
    }
}

fn register_set_price(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "PRICE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let product_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let price_list = context.eval_expression_tree(&inputs[1])?.to_string();
                let price_expr = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("SET PRICE: {product_id} list={price_list} price={price_expr}");
                let request = InventoryRequest::set_price(&product_id, &price_list, &price_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("SET PRICE syntax failed: {e}");
    }
}

fn register_open_pos_session(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["OPEN", "POS", "SESSION", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let branch_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let opening_expr = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("OPEN POS SESSION: branch={branch_id} opening={opening_expr}");
                let request = InventoryRequest::open_pos_session(&branch_id, &opening_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("OPEN POS SESSION syntax failed: {e}");
    }
}

fn register_add_to_cart(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["ADD", "TO", "CART", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let product_id = context.eval_expression_tree(&inputs[1])?.to_string();
                let qty_expr = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("ADD TO CART: session={session_id} product={product_id} qty={qty_expr}");
                let request = InventoryRequest::add_to_cart(&session_id, &product_id, &qty_expr);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("ADD TO CART syntax failed: {e}");
    }
}

fn register_checkout(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["CHECKOUT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let payment_method = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("CHECKOUT: session={session_id} payment={payment_method}");
                let request = InventoryRequest::checkout(&session_id, &payment_method);
                Ok(serde_json_to_dynamic(&request))
            },
        )
    {
        log::error!("CHECKOUT syntax failed: {e}");
    }
}

fn serde_json_to_dynamic<T: serde::Serialize>(value: &T) -> Dynamic {
    match serde_json::to_value(value) {
        Ok(json) => Dynamic::from(json.to_string()),
        Err(e) => {
            log::error!("inventory request serialization failed: {e}");
            Dynamic::UNIT
        }
    }
}
