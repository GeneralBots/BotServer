use super::*;

impl InvoiceService {
    pub async fn add_line_item(
        &self,
        invoice: &mut Invoice,
        item: CreateLineItemRequest,
    ) -> Result<(), InvoiceError> {
        if invoice.status != InvoiceStatus::Draft {
            return Err(InvoiceError::InvalidStatus(
                "Can only add items to draft invoices".to_string(),
            ));
        }

        let amount = (item.quantity * item.unit_price as f64) as i64;

        let line_item = InvoiceLineItem {
            id: Uuid::new_v4(),
            description: item.description,
            quantity: item.quantity,
            unit_price: item.unit_price,
            amount,
            currency: invoice.currency.clone(),
            period_start: item.period_start,
            period_end: item.period_end,
            proration: item.proration,
            metadata: item.metadata.unwrap_or_default(),
        };

        invoice.line_items.push(line_item);
        self.recalculate_totals(invoice);

        Ok(())
    }

    pub async fn remove_line_item(
        &self,
        invoice: &mut Invoice,
        item_id: Uuid,
    ) -> Result<(), InvoiceError> {
        if invoice.status != InvoiceStatus::Draft {
            return Err(InvoiceError::InvalidStatus(
                "Can only remove items from draft invoices".to_string(),
            ));
        }

        let initial_len = invoice.line_items.len();
        invoice.line_items.retain(|item| item.id != item_id);

        if invoice.line_items.len() == initial_len {
            return Err(InvoiceError::NotFound(
                "Line item not found".to_string(),
            ));
        }

        self.recalculate_totals(invoice);
        Ok(())
    }

    pub async fn apply_discount(
        &self,
        invoice: &mut Invoice,
        discount: CreateDiscountRequest,
    ) -> Result<(), InvoiceError> {
        if invoice.status != InvoiceStatus::Draft {
            return Err(InvoiceError::InvalidStatus(
                "Can only apply discounts to draft invoices".to_string(),
            ));
        }

        let amount = if let Some(percent) = discount.percent_off {
            (invoice.subtotal as f64 * percent / 100.0) as i64
        } else {
            discount.amount.unwrap_or(0)
        };

        let invoice_discount = InvoiceDiscount {
            id: Uuid::new_v4(),
            coupon_id: discount.coupon_id,
            description: discount.description,
            amount,
            percent_off: discount.percent_off,
        };

        invoice.discounts.push(invoice_discount);
        self.recalculate_totals(invoice);

        Ok(())
    }

    pub(super) fn recalculate_totals(&self, invoice: &mut Invoice) {
        invoice.subtotal = invoice.line_items.iter().map(|item| item.amount).sum();

        for discount in &mut invoice.discounts {
            if let Some(percent) = discount.percent_off {
                discount.amount = (invoice.subtotal as f64 * percent / 100.0) as i64;
            }
        }

        invoice.total_discount = invoice.discounts.iter().map(|d| d.amount).sum();
        let taxable_amount = invoice.subtotal - invoice.total_discount;

        for tax in &mut invoice.taxes {
            tax.amount = (taxable_amount as f64 * tax.rate / 100.0) as i64;
        }

        invoice.total_tax = invoice.taxes.iter().map(|t| t.amount).sum();
        invoice.total = taxable_amount + invoice.total_tax;
        invoice.amount_due = invoice.total - invoice.amount_paid;
        invoice.updated_at = Utc::now();
    }

    pub fn check_overdue(&self, invoice: &mut Invoice) {
        if invoice.status == InvoiceStatus::Open && Utc::now() > invoice.due_date {
            invoice.status = InvoiceStatus::PastDue;
            invoice.updated_at = Utc::now();
        }
    }

    pub async fn generate_pdf(&self, invoice: &Invoice) -> Result<Vec<u8>, InvoiceError> {
        let html = self.generate_invoice_html(invoice);
        let pdf_bytes = self.html_to_pdf(&html).await?;
        Ok(pdf_bytes)
    }
}
