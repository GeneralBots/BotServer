use super::*;

impl InvoiceService {
    pub(super) fn generate_invoice_html(&self, invoice: &Invoice) -> String {
        let line_items_html: String = invoice
            .line_items
            .iter()
            .map(|item| {
                format!(
                    r#"<tr>
<td>{}</td>
<td class="right">{:.2}</td>
<td class="right">{}</td>
<td class="right">{}</td>
</tr>"#,
                    item.description,
                    item.quantity,
                    self.format_currency(item.unit_price, &invoice.currency),
                    self.format_currency(item.amount, &invoice.currency)
                )
            })
            .collect();

        let discounts_html: String = invoice
            .discounts
            .iter()
            .map(|d| {
                format!(
                    r#"<tr class="discount">
<td colspan="3">{}</td>
<td class="right">-{}</td>
</tr>"#,
                    d.description,
                    self.format_currency(d.amount, &invoice.currency)
                )
            })
            .collect();

        let taxes_html: String = invoice
            .taxes
            .iter()
            .map(|t| {
                format!(
                    r#"<tr class="tax">
<td colspan="3">{}</td>
<td class="right">{}</td>
</tr>"#,
                    t.description,
                    self.format_currency(t.amount, &invoice.currency)
                )
            })
            .collect();

        let status_class = match invoice.status {
            InvoiceStatus::Paid => "status-paid",
            InvoiceStatus::PastDue => "status-overdue",
            InvoiceStatus::Void => "status-void",
            _ => "status-open",
        };

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<title>Invoice {}</title>
<style>
body {{ font-family: 'Helvetica Neue', Arial, sans-serif; margin: 0; padding: 40px; color: #333; }}
.header {{ display: flex; justify-content: space-between; margin-bottom: 40px; }}
.company-info {{ font-size: 14px; line-height: 1.6; }}
.company-name {{ font-size: 24px; font-weight: bold; margin-bottom: 8px; }}
.invoice-info {{ text-align: right; }}
.invoice-number {{ font-size: 28px; font-weight: bold; color: #2563eb; }}
.invoice-date {{ font-size: 14px; color: #666; margin-top: 8px; }}
.status {{ display: inline-block; padding: 4px 12px; border-radius: 4px; font-size: 12px; font-weight: bold; text-transform: uppercase; }}
.status-paid {{ background: #dcfce7; color: #166534; }}
.status-open {{ background: #dbeafe; color: #1e40af; }}
.status-overdue {{ background: #fee2e2; color: #991b1b; }}
.status-void {{ background: #f3f4f6; color: #6b7280; }}
.customer-section {{ margin-bottom: 40px; padding: 20px; background: #f9fafb; border-radius: 8px; }}
.customer-section h3 {{ margin: 0 0 12px 0; font-size: 12px; text-transform: uppercase; color: #666; }}
.customer-name {{ font-size: 18px; font-weight: bold; }}
.customer-details {{ font-size: 14px; color: #666; line-height: 1.6; }}
table {{ width: 100%; border-collapse: collapse; margin-bottom: 20px; }}
th {{ background: #f3f4f6; padding: 12px; text-align: left; font-size: 12px; text-transform: uppercase; color: #666; }}
td {{ padding: 12px; border-bottom: 1px solid #e5e7eb; }}
.right {{ text-align: right; }}
.discount td {{ color: #16a34a; }}
.tax td {{ color: #666; }}
.totals {{ margin-top: 20px; }}
.totals table {{ width: 300px; margin-left: auto; }}
.totals td {{ padding: 8px 0; }}
.totals .total-row {{ font-size: 18px; font-weight: bold; border-top: 2px solid #333; }}
.notes {{ margin-top: 40px; padding: 20px; background: #fffbeb; border-radius: 8px; font-size: 14px; }}
.footer {{ margin-top: 40px; padding-top: 20px; border-top: 1px solid #e5e7eb; font-size: 12px; color: #666; text-align: center; }}
</style>
</head>
<body>
<div class="header">
<div class="company-info">
<div class="company-name">{}</div>
<div>{}</div>
<div>{}, {} {}</div>
<div>{}</div>
{}
</div>
<div class="invoice-info">
<div class="invoice-number">Invoice {}</div>
<div class="invoice-date">Date: {}</div>
<div class="invoice-date">Due: {}</div>
<div style="margin-top: 12px;"><span class="status {}">{:?}</span></div>
</div>
</div>

<div class="customer-section">
<h3>Bill To</h3>
<div class="customer-name">{}</div>
<div class="customer-details">
{}
{}
</div>
</div>

<table>
<thead>
<tr>
<th>Description</th>
<th class="right">Quantity</th>
<th class="right">Unit Price</th>
<th class="right">Amount</th>
</tr>
</thead>
<tbody>
{}
{}
{}
</tbody>
</table>

<div class="totals">
<table>
<tr>
<td>Subtotal</td>
<td class="right">{}</td>
</tr>
<tr>
<td>Discount</td>
<td class="right">-{}</td>
</tr>
<tr>
<td>Tax</td>
<td class="right">{}</td>
</tr>
<tr class="total-row">
<td>Total</td>
<td class="right">{}</td>
</tr>
<tr>
<td>Amount Paid</td>
<td class="right">{}</td>
</tr>
<tr class="total-row">
<td>Amount Due</td>
<td class="right">{}</td>
</tr>
</table>
</div>

{}

<div class="footer">
{}
</div>
</body>
</html>"#,
            invoice.number,
            self.company_name,
            self.company_address.line1.as_deref().unwrap_or(""),
            self.company_address.city.as_deref().unwrap_or(""),
            self.company_address.state.as_deref().unwrap_or(""),
            self.company_address.postal_code.as_deref().unwrap_or(""),
            self.company_address.country,
            self.company_tax_id.as_ref().map(|id| format!("<div>Tax ID: {}</div>", id)).unwrap_or_default(),
            invoice.number,
            invoice.created_at.format("%B %d, %Y"),
            invoice.due_date.format("%B %d, %Y"),
            status_class,
            invoice.status,
            invoice.customer.name,
            invoice.customer.email,
            invoice.customer.address.as_ref().map(|a| {
                format!("<div>{}, {} {} {}</div>",
                    a.line1.as_deref().unwrap_or(""),
                    a.city.as_deref().unwrap_or(""),
                    a.state.as_deref().unwrap_or(""),
                    a.postal_code.as_deref().unwrap_or("")
                )
            }).unwrap_or_default(),
            line_items_html,
            discounts_html,
            taxes_html,
            self.format_currency(invoice.subtotal, &invoice.currency),
            self.format_currency(invoice.total_discount, &invoice.currency),
            self.format_currency(invoice.total_tax, &invoice.currency),
            self.format_currency(invoice.total, &invoice.currency),
            self.format_currency(invoice.amount_paid, &invoice.currency),
            self.format_currency(invoice.amount_due, &invoice.currency),
            invoice.notes.as_ref().map(|n| format!(r#"<div class="notes"><strong>Notes:</strong><br>{}</div>"#, n)).unwrap_or_default(),
            invoice.footer.as_deref().unwrap_or("Thank you for your business!")
        )
    }

    pub(super) fn format_currency(&self, amount: i64, currency: &str) -> String {
        let symbol = match currency.to_lowercase().as_str() {
            "usd" => "$",
            "eur" => "\u{20ac}",
            "gbp" => "\u{a3}",
            "jpy" => "\u{a5}",
            "brl" => "R$",
            _ => currency,
        };

        let decimal_places = match currency.to_lowercase().as_str() {
            "jpy" => 0,
            _ => 2,
        };

        if decimal_places == 0 {
            format!("{}{}", symbol, amount)
        } else {
            let dollars = amount / 100;
            let cents = (amount % 100).abs();
            format!("{}{}.{:02}", symbol, dollars, cents)
        }
    }

    pub(crate) async fn html_to_pdf(&self, _html: &str) -> Result<Vec<u8>, InvoiceError> {
        Ok(Vec::new())
    }

    pub(crate) async fn create_stripe_invoice(
        &self,
        invoice: &Invoice,
        _stripe_key: &str,
    ) -> Result<StripeInvoiceResult, InvoiceError> {
        Ok(StripeInvoiceResult {
            id: format!("in_{}", invoice.id),
            hosted_url: Some(format!("https://invoice.stripe.com/i/{}", invoice.id)),
            pdf_url: Some(format!("https://invoice.stripe.com/i/{}/pdf", invoice.id)),
        })
    }
}
