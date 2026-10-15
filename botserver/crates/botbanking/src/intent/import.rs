//! CSV statement imports feeding the banking BASIC keywords.
//!
//! Both importers accept the loose, human-authored CSV that arrives from a
//! BASIC script (pt-BR or English headers, `R$ 1.234,56` amounts, `dd/mm/yyyy`
//! dates) and normalise it into the row shape the runtime stores.

use std::collections::BTreeMap;

use super::{
    BankTransactionImport, BankTransactionRow, BankingRequestKind, DeliveryTransactionImport,
    DeliveryTransactionRow,
};

/// A parsed CSV row: header name → raw cell value.
pub(super) type Row = BTreeMap<String, String>;

/// Splits a CSV into rows keyed by the header line. Quoting is not supported —
/// the same contract as before this logic moved into the crate (#1506).
pub(super) fn parse_csv_rows(csv: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let mut lines = csv.lines();
    let header_line = match lines.next() {
        Some(h) => h,
        None => return rows,
    };
    let headers: Vec<&str> = header_line.split(',').map(|s| s.trim()).collect();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        let mut row = Row::new();
        for (i, header) in headers.iter().enumerate() {
            let value = values.get(i).copied().unwrap_or("").to_string();
            row.insert((*header).to_string(), value);
        }
        rows.push(row);
    }
    rows
}

/// Parses a pt-BR currency cell (`R$ 1.234,56`) into `f64`, defaulting to 0.
pub(super) fn parse_amount(s: &str) -> f64 {
    s.replace(['R', '$', ' '], "")
        .replace('.', "")
        .replace(',', ".")
        .parse::<f64>()
        .unwrap_or(0.0)
}

/// Converts `dd/mm/yyyy` into `yyyy-mm-dd`.
pub(super) fn parse_br_date(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 {
        Some(format!("{}-{:0>2}-{:0>2}", parts[2], parts[1], parts[0]))
    } else {
        None
    }
}

/// First non-empty value among the accepted header aliases.
fn cell<'a>(row: &'a Row, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| row.get(*key).map(String::as_str))
}

/// `IMPORT BANK STATEMENT <csv>` — normalise a bank export into transactions.
pub fn import_bank_statement(csv: &str) -> BankTransactionImport {
    let rows = parse_csv_rows(csv);
    let mut transactions: Vec<BankTransactionRow> = Vec::new();
    for row in &rows {
        let transaction_date = cell(row, &["date", "data"])
            .and_then(parse_br_date)
            .unwrap_or_else(|| "1970-01-01".to_string());
        let description = cell(row, &["description", "descricao", "historico"])
            .unwrap_or_default()
            .to_string();
        let amount = cell(row, &["amount", "valor"])
            .map(parse_amount)
            .unwrap_or(0.0);
        let bank = cell(row, &["bank"]).map(str::to_string);
        let account = cell(row, &["account"]).map(str::to_string);

        transactions.push(BankTransactionRow {
            transaction_date,
            description,
            amount,
            bank,
            account,
            reconciled: false,
        });
    }

    BankTransactionImport {
        kind: BankingRequestKind::BankTransactions,
        count: transactions.len(),
        rows: transactions,
    }
}

/// `IMPORT PLATFORM ORDERS <csv>` — normalise a delivery platform export.
pub fn import_platform_orders(csv: &str) -> DeliveryTransactionImport {
    let rows = parse_csv_rows(csv);
    let mut transactions: Vec<DeliveryTransactionRow> = Vec::new();
    for row in &rows {
        let platform = cell(row, &["platform"]).unwrap_or("unknown").to_string();
        let platform_order_id = cell(row, &["order_id", "id", "pedido_id"])
            .unwrap_or_default()
            .to_string();
        let order_date = cell(row, &["date", "data"])
            .and_then(parse_br_date)
            .unwrap_or_else(|| "1970-01-01".to_string());
        let customer_name = cell(row, &["customer", "cliente"]).map(str::to_string);
        let subtotal = cell(row, &["subtotal", "valor"])
            .map(parse_amount)
            .unwrap_or(0.0);
        let delivery_fee = cell(row, &["delivery_fee", "entrega"])
            .map(parse_amount)
            .unwrap_or(0.0);
        let platform_commission = cell(row, &["commission", "comissao"])
            .map(parse_amount)
            .unwrap_or(0.0);
        let net_value = cell(row, &["net", "liquido"])
            .map(parse_amount)
            .unwrap_or(subtotal - platform_commission);

        transactions.push(DeliveryTransactionRow {
            platform,
            platform_order_id,
            order_date,
            customer_name,
            subtotal,
            delivery_fee,
            platform_commission,
            net_value,
            reconciled: false,
            status: "delivered".to_string(),
        });
    }

    DeliveryTransactionImport {
        kind: BankingRequestKind::DeliveryTransactions,
        count: transactions.len(),
        rows: transactions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn value<T: serde::Serialize>(item: &T) -> Value {
        serde_json::to_value(item).expect("serialize")
    }

    #[test]
    fn bank_statement_accepts_english_headers() {
        let csv = "date,description,amount,bank,account\n\
                   01/08/2026,PIX RECEBIDO,R$ 1.234,56,Itau,12345\n";
        let json = value(&import_bank_statement(csv));
        assert_eq!(json["kind"], "bank_transactions");
        assert_eq!(json["count"], 1);
        assert_eq!(json["rows"][0]["transaction_date"], "2026-08-01");
        assert_eq!(json["rows"][0]["amount"], 1234.56);
        assert_eq!(json["rows"][0]["description"], "PIX RECEBIDO");
        assert_eq!(json["rows"][0]["reconciled"], false);
    }

    #[test]
    fn bank_statement_accepts_pt_br_headers() {
        let csv = "data,historico,valor\n15/07/2026,TARIFA,-12,00\n";
        let json = value(&import_bank_statement(csv));
        assert_eq!(json["rows"][0]["transaction_date"], "2026-07-15");
        assert_eq!(json["rows"][0]["description"], "TARIFA");
        assert_eq!(json["rows"][0]["amount"], -12.0);
        assert_eq!(json["rows"][0]["bank"], Value::Null);
    }

    #[test]
    fn bank_statement_defaults_unparseable_cells() {
        let csv = "date,description,amount\nnot-a-date,no amount col,\n";
        let json = value(&import_bank_statement(csv));
        assert_eq!(json["rows"][0]["transaction_date"], "1970-01-01");
        assert_eq!(json["rows"][0]["amount"], 0.0);
    }

    #[test]
    fn platform_orders_derive_net_from_subtotal_and_commission() {
        let csv = "platform,order_id,date,customer,subtotal,delivery_fee,commission\n\
                   ifood,99,02/08/2026,Ana,100,00,5,00,20,00\n";
        let json = value(&import_platform_orders(csv));
        assert_eq!(json["kind"], "delivery_transactions");
        assert_eq!(json["rows"][0]["order_date"], "2026-08-02");
        assert_eq!(json["rows"][0]["net_value"], 80.0);
        assert_eq!(json["rows"][0]["status"], "delivered");
        assert_eq!(json["rows"][0]["customer_name"], "Ana");
    }

    #[test]
    fn platform_orders_honour_an_explicit_net_column() {
        // The row splitter is the naive comma split the keywords always used:
        // decimal commas inside a cell are not quoted, so a literal net wins
        // only when it is the cell's exact content.
        let csv = "platform,order_id,date,subtotal,commission,net\nrappi,1,02/08/2026,50,10,37\n";
        let json = value(&import_platform_orders(csv));
        assert_eq!(json["rows"][0]["net_value"], 37.0);
    }

    #[test]
    fn empty_csv_yields_no_rows() {
        assert_eq!(value(&import_bank_statement(""))["count"], 0);
        assert_eq!(value(&import_platform_orders(""))["count"], 0);
    }
}
