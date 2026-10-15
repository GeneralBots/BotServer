//! Canonical request payloads for the inventory and POS BASIC keywords.
//!
//! `CREATE PRODUCT`, `ADD VARIATION`, `SET STOCK`, `TRANSFER STOCK`,
//! `GET STOCK`, `SET PRICE`, `OPEN POS SESSION`, `ADD TO CART` and
//! `CHECKOUT` only bind arguments; the shape of the command they emit — and
//! the parsing of its numeric arguments — belongs to this crate (#1506).

use serde::{Deserialize, Serialize};

/// What an inventory/POS request asks for. Serializes to the `kind` field
/// the runtime routes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryRequestKind {
    Product,
    ProductVariation,
    ProductStock,
    InventoryMovement,
    StockQuery,
    ProductPrice,
    PosSession,
    CartItem,
    PosSale,
}

impl InventoryRequestKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Product => "product",
            Self::ProductVariation => "product_variation",
            Self::ProductStock => "product_stock",
            Self::InventoryMovement => "inventory_movement",
            Self::StockQuery => "stock_query",
            Self::ProductPrice => "product_price",
            Self::PosSession => "pos_session",
            Self::CartItem => "cart_item",
            Self::PosSale => "pos_sale",
        }
    }
}

/// Parses a numeric argument coming from a BASIC script, falling back to
/// `default` when the expression is not a number.
pub fn parse_amount(raw: &str, default: f64) -> f64 {
    raw.trim().parse::<f64>().unwrap_or(default)
}

/// An inventory/POS command ready to be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRequest {
    pub kind: InventoryRequestKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_list_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opening_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub movement_type: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_branch_id: Option<String>,
}

impl InventoryRequest {
    fn empty(kind: InventoryRequestKind) -> Self {
        Self {
            kind,
            action: None,
            name: None,
            sku: None,
            product_id: None,
            branch_id: None,
            session_id: None,
            price: None,
            quantity: None,
            price_list_name: None,
            opening_amount: None,
            payment_method: None,
            is_active: None,
            status: None,
            movement_type: None,
            from_branch_id: None,
            to_branch_id: None,
        }
    }

    /// `CREATE PRODUCT <name>, <sku>, <price>`
    pub fn create_product(name: &str, sku: &str, price: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::Product);
        r.action = Some("create");
        r.name = Some(name.to_string());
        r.sku = Some(sku.to_string());
        r.price = Some(parse_amount(price, 0.0));
        r.is_active = Some(true);
        r
    }

    /// `ADD VARIATION <product>, <sku>, <name>, <price>`
    pub fn add_variation(product_id: &str, sku: &str, name: &str, price: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::ProductVariation);
        r.action = Some("create");
        r.product_id = Some(product_id.to_string());
        r.sku = Some(sku.to_string());
        r.name = Some(name.to_string());
        r.price = Some(parse_amount(price, 0.0));
        r.is_active = Some(true);
        r
    }

    /// `SET STOCK <product>, <branch>, <quantity>`
    pub fn set_stock(product_id: &str, branch_id: &str, quantity: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::ProductStock);
        r.action = Some("upsert");
        r.product_id = Some(product_id.to_string());
        r.branch_id = Some(branch_id.to_string());
        r.quantity = Some(parse_amount(quantity, 0.0));
        r
    }

    /// `TRANSFER STOCK <product>, <from>, <to>, <quantity>`
    pub fn transfer_stock(product_id: &str, from_branch: &str, to_branch: &str, quantity: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::InventoryMovement);
        r.movement_type = Some("transfer");
        r.product_id = Some(product_id.to_string());
        r.from_branch_id = Some(from_branch.to_string());
        r.to_branch_id = Some(to_branch.to_string());
        r.quantity = Some(parse_amount(quantity, 0.0));
        r
    }

    /// `GET STOCK <product>`
    pub fn get_stock(product_id: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::StockQuery);
        r.product_id = Some(product_id.to_string());
        r
    }

    /// `SET PRICE <product>, <price list>, <price>`
    pub fn set_price(product_id: &str, price_list: &str, price: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::ProductPrice);
        r.action = Some("upsert");
        r.product_id = Some(product_id.to_string());
        r.price_list_name = Some(price_list.to_string());
        r.price = Some(parse_amount(price, 0.0));
        r
    }

    /// `OPEN POS SESSION <branch>, <opening amount>`
    pub fn open_pos_session(branch_id: &str, opening_amount: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::PosSession);
        r.action = Some("open");
        r.branch_id = Some(branch_id.to_string());
        r.opening_amount = Some(parse_amount(opening_amount, 0.0));
        r.status = Some("open");
        r
    }

    /// `ADD TO CART <session>, <product>, <quantity>`
    pub fn add_to_cart(session_id: &str, product_id: &str, quantity: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::CartItem);
        r.session_id = Some(session_id.to_string());
        r.product_id = Some(product_id.to_string());
        r.quantity = Some(parse_amount(quantity, 1.0));
        r
    }

    /// `CHECKOUT <session>, <payment method>`
    pub fn checkout(session_id: &str, payment_method: &str) -> Self {
        let mut r = Self::empty(InventoryRequestKind::PosSale);
        r.action = Some("create");
        r.session_id = Some(session_id.to_string());
        r.payment_method = Some(payment_method.to_string());
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_product_parses_price_and_activates() {
        let json = serde_json::to_value(InventoryRequest::create_product("Widget", "W-1", "12.50")).unwrap();
        assert_eq!(json["kind"], "product");
        assert_eq!(json["action"], "create");
        assert_eq!(json["price"], 12.5);
        assert_eq!(json["is_active"], true);
    }

    #[test]
    fn non_numeric_price_falls_back_to_zero() {
        let json = serde_json::to_value(InventoryRequest::create_product("Widget", "W-1", "abc")).unwrap();
        assert_eq!(json["price"], 0.0);
    }

    #[test]
    fn cart_quantity_defaults_to_one() {
        let json = serde_json::to_value(InventoryRequest::add_to_cart("s1", "p1", "")).unwrap();
        assert_eq!(json["quantity"], 1.0);
        assert_eq!(json["kind"], "cart_item");
    }

    #[test]
    fn transfer_records_both_branches() {
        let json = serde_json::to_value(InventoryRequest::transfer_stock("p1", "b1", "b2", "3"))
            .unwrap();
        assert_eq!(json["movement_type"], "transfer");
        assert_eq!(json["from_branch_id"], "b1");
        assert_eq!(json["to_branch_id"], "b2");
    }

    #[test]
    fn pos_session_reports_open_status() {
        let json = serde_json::to_value(InventoryRequest::open_pos_session("b1", "100")).unwrap();
        assert_eq!(json["status"], "open");
        assert_eq!(json["opening_amount"], 100.0);
    }
}
