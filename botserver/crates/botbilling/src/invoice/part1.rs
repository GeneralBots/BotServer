use super::*;

impl InvoiceService {
    pub fn new(
        stripe_secret_key: Option<String>,
        company_name: String,
        company_address: CustomerAddress,
    ) -> Self {
        Self {
            stripe_secret_key,
            company_name,
            company_address,
            company_tax_id: None,
            default_currency: "usd".to_string(),
            default_due_days: 30,
            invoice_prefix: "INV".to_string(),
            next_invoice_number: std::sync::atomic::AtomicU64::new(1000),
        }
    }

    pub fn with_tax_id(mut self, tax_id: String) -> Self {
        self.company_tax_id = Some(tax_id);
        self
    }

    pub fn with_default_currency(mut self, currency: String) -> Self {
        self.default_currency = currency;
        self
    }

    pub fn with_due_days(mut self, days: i64) -> Self {
        self.default_due_days = days;
        self
    }

    pub fn with_invoice_prefix(mut self, prefix: String) -> Self {
        self.invoice_prefix = prefix;
        self
    }

    pub(super) fn generate_invoice_number(&self) -> String {
        let num = self
            .next_invoice_number
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let now = Utc::now();
        format!(
            "{}-{}{:02}-{:05}",
            self.invoice_prefix,
            now.year(),
            now.month(),
            num
        )
    }

    pub async fn create_invoice(
        &self,
        request: CreateInvoiceRequest,
    ) -> Result<Invoice, InvoiceError> {
        let now = Utc::now();
        let invoice_id = Uuid::new_v4();
        let invoice_number = self.generate_invoice_number();

        let line_items: Vec<InvoiceLineItem> = request
            .line_items
            .into_iter()
            .map(|item| {
                let amount = (item.quantity * item.unit_price as f64) as i64;
                InvoiceLineItem {
                    id: Uuid::new_v4(),
                    description: item.description,
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    amount,
                    currency: request.currency.clone(),
                    period_start: item.period_start,
                    period_end: item.period_end,
                    proration: item.proration,
                    metadata: item.metadata.unwrap_or_default(),
                }
            })
            .collect();

        let subtotal: i64 = line_items.iter().map(|item| item.amount).sum();

        let discounts: Vec<InvoiceDiscount> = request
            .discounts
            .unwrap_or_default()
            .into_iter()
            .map(|d| {
                let amount = if let Some(percent) = d.percent_off {
                    (subtotal as f64 * percent / 100.0) as i64
                } else {
                    d.amount.unwrap_or(0)
                };
                InvoiceDiscount {
                    id: Uuid::new_v4(),
                    coupon_id: d.coupon_id,
                    description: d.description,
                    amount,
                    percent_off: d.percent_off,
                }
            })
            .collect();

        let total_discount: i64 = discounts.iter().map(|d| d.amount).sum();
        let taxable_amount = subtotal - total_discount;

        let taxes: Vec<InvoiceTax> = if let Some(rate) = request.tax_rate {
            vec![InvoiceTax {
                id: Uuid::new_v4(),
                description: format!("Tax ({:.2}%)", rate),
                rate,
                amount: (taxable_amount as f64 * rate / 100.0) as i64,
                jurisdiction: None,
                tax_type: "sales_tax".to_string(),
            }]
        } else {
            vec![]
        };

        let total_tax: i64 = taxes.iter().map(|t| t.amount).sum();
        let total = taxable_amount + total_tax;

        let due_days = request.due_days.unwrap_or(self.default_due_days);
        let due_date = now + chrono::Duration::days(due_days);

        let invoice = Invoice {
            id: invoice_id,
            number: invoice_number,
            organization_id: request.organization_id,
            customer: request.customer,
            status: InvoiceStatus::Draft,
            invoice_type: request.invoice_type,
            currency: request.currency,
            line_items,
            discounts,
            taxes,
            subtotal,
            total_discount,
            total_tax,
            total,
            amount_paid: 0,
            amount_due: total,
            period_start: now,
            period_end: due_date,
            due_date,
            paid_at: None,
            voided_at: None,
            created_at: now,
            updated_at: now,
            payment_attempts: vec![],
            stripe_invoice_id: None,
            hosted_invoice_url: None,
            pdf_url: None,
            notes: request.notes,
            footer: request.footer,
            metadata: request.metadata.unwrap_or_default(),
        };

        Ok(invoice)
    }

    pub async fn finalize_invoice(
        &self,
        invoice: &mut Invoice,
    ) -> Result<(), InvoiceError> {
        if invoice.status != InvoiceStatus::Draft {
            return Err(InvoiceError::InvalidStatus(
                "Only draft invoices can be finalized".to_string(),
            ));
        }

        invoice.status = InvoiceStatus::Open;
        invoice.updated_at = Utc::now();

        if let Some(ref stripe_key) = self.stripe_secret_key {
            let stripe_invoice = self.create_stripe_invoice(invoice, stripe_key).await?;
            invoice.stripe_invoice_id = Some(stripe_invoice.id);
            invoice.hosted_invoice_url = stripe_invoice.hosted_url;
            invoice.pdf_url = stripe_invoice.pdf_url;
        }

        Ok(())
    }

    pub async fn mark_as_paid(
        &self,
        invoice: &mut Invoice,
        payment_method: PaymentMethod,
    ) -> Result<(), InvoiceError> {
        if invoice.status == InvoiceStatus::Paid {
            return Err(InvoiceError::InvalidStatus(
                "Invoice is already paid".to_string(),
            ));
        }

        if invoice.status == InvoiceStatus::Void {
            return Err(InvoiceError::InvalidStatus(
                "Cannot mark voided invoice as paid".to_string(),
            ));
        }

        let now = Utc::now();

        invoice.payment_attempts.push(PaymentAttempt {
            id: Uuid::new_v4(),
            timestamp: now,
            amount: invoice.amount_due,
            status: "succeeded".to_string(),
            failure_reason: None,
            payment_method,
        });

        invoice.amount_paid = invoice.total;
        invoice.amount_due = 0;
        invoice.status = InvoiceStatus::Paid;
        invoice.paid_at = Some(now);
        invoice.updated_at = now;

        Ok(())
    }

    pub async fn void_invoice(
        &self,
        invoice: &mut Invoice,
        reason: Option<String>,
    ) -> Result<(), InvoiceError> {
        if invoice.status == InvoiceStatus::Paid {
            return Err(InvoiceError::InvalidStatus(
                "Cannot void a paid invoice. Use refund instead.".to_string(),
            ));
        }

        if invoice.status == InvoiceStatus::Void {
            return Err(InvoiceError::InvalidStatus(
                "Invoice is already voided".to_string(),
            ));
        }

        let now = Utc::now();
        invoice.status = InvoiceStatus::Void;
        invoice.voided_at = Some(now);
        invoice.updated_at = now;

        if let Some(note) = reason {
            let existing_notes = invoice.notes.take().unwrap_or_default();
            invoice.notes = Some(format!("{}\n\nVoided: {}", existing_notes, note));
        }

        Ok(())
    }

    pub async fn refund_invoice(
        &self,
        invoice: &mut Invoice,
        amount: Option<i64>,
        reason: Option<String>,
    ) -> Result<RefundResult, InvoiceError> {
        if invoice.status != InvoiceStatus::Paid {
            return Err(InvoiceError::InvalidStatus(
                "Only paid invoices can be refunded".to_string(),
            ));
        }

        let refund_amount = amount.unwrap_or(invoice.amount_paid);

        if refund_amount > invoice.amount_paid {
            return Err(InvoiceError::InvalidAmount(
                "Refund amount exceeds paid amount".to_string(),
            ));
        }

        let now = Utc::now();
        invoice.amount_paid -= refund_amount;
        invoice.amount_due += refund_amount;
        invoice.updated_at = now;

        if invoice.amount_paid == 0 {
            invoice.status = InvoiceStatus::Refunded;
        } else {
            invoice.status = InvoiceStatus::PartiallyRefunded;
        }

        let refund_result = RefundResult {
            id: Uuid::new_v4(),
            invoice_id: invoice.id,
            amount: refund_amount,
            currency: invoice.currency.clone(),
            reason,
            created_at: now,
        };

        Ok(refund_result)
    }
}
