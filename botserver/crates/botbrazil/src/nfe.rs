//! NFe (Nota Fiscal eletrônica) building block. Validates the document body
//! before sending it to the SEFAZ web service. No real network call is
//! performed — that is delegated to the integration layer that owns the
//! digital certificate.

use crate::models::{DocumentStatus, FiscalDocument};
use crate::validators::{is_valid_cnpj, is_valid_nfe_access_key, TaxError};

pub struct NFeDraft {
    pub document: FiscalDocument,
    pub items_total: f64,
    pub tax_total: f64,
}

pub fn build_nfe(mut document: FiscalDocument) -> Result<NFeDraft, TaxError> {
    if !is_valid_cnpj(&document.issuer.tax_id) {
        return Err(TaxError::InvalidCnpj(document.issuer.tax_id.clone()));
    }
    if !document.recipient.tax_id.is_empty() && !is_valid_cnpj(&document.recipient.tax_id) {
        return Err(TaxError::InvalidCnpj(document.recipient.tax_id.clone()));
    }
    if document.document_number.is_empty() {
        return Err(TaxError::MissingField("document_number"));
    }
    if document.series.is_empty() {
        return Err(TaxError::MissingField("series"));
    }
    if document.items.is_empty() {
        return Err(TaxError::MissingField("items"));
    }
    if document.status != DocumentStatus::Draft {
        return Err(TaxError::InvalidState(format!("{:?}", document.status)));
    }
    document.recalculate_totals();
    Ok(NFeDraft {
        items_total: document.total_products,
        tax_total: document.total_taxes,
        document,
    })
}

/// Maps one character of the 43 significant positions onto its numeric value
/// for the modulo 11 check digit. Digits keep their value and letters continue
/// the sequence at 10 (`A` = 10 ... `Z` = 35), as specified for the NFe access
/// key; anything else counts as zero.
///
/// The previous implementation subtracted `b'0'` from every byte, which
/// overflowed on any letter and aborted the build.
fn key_digit_value(character: char) -> u32 {
    match character {
        '0'..='9' => character as u32 - '0' as u32,
        'A'..='Z' => character as u32 - 'A' as u32 + 10,
        'a'..='z' => character as u32 - 'a' as u32 + 10,
        _ => 0,
    }
}

/// Builds the 44-digit NFe access key (chave de acesso).
///
/// Layout, one digit per position: cUF (2) + AAMM (4) + CNPJ (14) + mod (2) +
/// série (3) + nNF (9) + tpEmis (1) + cNF (8) + cDV (1).
pub fn build_access_key(
    state: &str,
    issue_year: u16,
    issue_month: u8,
    cnpj: &str,
    model: &str,
    series: &str,
    number: u32,
    emission_kind: &str,
    code: u32,
) -> Result<String, TaxError> {
    let cnpj_digits = cnpj.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    if cnpj_digits.len() != 14 {
        return Err(TaxError::InvalidCnpj(cnpj.to_string()));
    }

    let state_digits = state.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    if state_digits.len() != 2 {
        return Err(TaxError::InvalidAccessKey(state.to_string()));
    }

    let model_digits = model.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    if model_digits.len() != 2 {
        return Err(TaxError::InvalidAccessKey(model.to_string()));
    }

    let kind_digit = emission_kind
        .chars()
        .find(|c| c.is_ascii_digit())
        .ok_or_else(|| TaxError::InvalidAccessKey(emission_kind.to_string()))?;

    if issue_month == 0 || issue_month > 12 {
        return Err(TaxError::InvalidAccessKey(issue_month.to_string()));
    }

    let series_number = series
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .unwrap_or(0);
    if series_number > 999 {
        return Err(TaxError::InvalidAccessKey(series.to_string()));
    }

    // 2 + 4 + 14 + 2 + 3 + 9 + 1 + 8 = 43 significant positions.
    let key = format!(
        "{}{:02}{:02}{}{}{:03}{:09}{}{:08}",
        state_digits,
        issue_year % 100,
        issue_month,
        cnpj_digits,
        model_digits,
        series_number,
        number,
        kind_digit,
        code,
    );

    let sum: u32 = key
        .chars()
        .enumerate()
        .map(|(index, character)| key_digit_value(character) * (43 - index) as u32)
        .sum();
    let remainder = sum % 11;
    let dv = if remainder < 2 { 0 } else { 11 - remainder };

    Ok(format!("{key}{dv}"))
}

pub fn validate_nfe_key(key: &str) -> Result<(), TaxError> {
    if !is_valid_nfe_access_key(key) {
        return Err(TaxError::InvalidAccessKey(key.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Party};

    fn party(tax_id: &str) -> Party {
        Party {
            tax_id: tax_id.into(),
            name: "Issuer".into(),
            legal_name: None,
            email: None,
            phone: None,
            address: Some(Address {
                street: "Av Paulista".into(),
                number: "1000".into(),
                complement: None,
                district: "Bela Vista".into(),
                city_code_ibge: "3550308".into(),
                city: "Sao Paulo".into(),
                state: "SP".into(),
                zip_code: "01310-100".into(),
                country_code: "1058".into(),
                country: "Brasil".into(),
            }),
            is_ie: true,
            is_icms_taxpayer: true,
        }
    }

    fn draft() -> FiscalDocument {
        let mut doc = FiscalDocument::new(party("11222333000181"), party(""), crate::models::DocumentKind::NFe);
        doc.document_number = "1".into();
        doc.series = "1".into();
        doc.add_item(crate::models::InvoiceItem {
            sku: "SKU1".into(),
            description: "Item 1".into(),
            ncm: Some("84715010".into()),
            cfop: Some("6102".into()),
            unit: "UN".into(),
            quantity: 1.0,
            unit_price: 100.0,
            total_price: 100.0,
            taxes: vec![],
            origin: 0,
        });
        doc
    }

    #[test]
    fn build_nfe_ok() {
        let draft = build_nfe(draft()).expect("valid NFe draft");
        assert_eq!(draft.items_total, 100.0);
    }

    #[test]
    fn build_nfe_rejects_invalid_cnpj() {
        let mut d = draft();
        d.issuer.tax_id = "123".into();
        let result = build_nfe(d);
        assert!(matches!(result, Err(TaxError::InvalidCnpj(_))));
    }

    #[test]
    fn build_nfe_rejects_missing_number() {
        let mut d = draft();
        d.document_number = String::new();
        let result = build_nfe(d);
        assert!(matches!(result, Err(TaxError::MissingField("document_number"))));
    }

    #[test]
    fn access_key_is_44_digits() {
        let key = build_access_key("35", 2026, 6, "11222333000181", "55", "1", 1, "1", 0).expect("key");
        assert_eq!(key.len(), 44);
        assert!(is_valid_nfe_access_key(&key));
    }
}
