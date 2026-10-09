use super::*;

impl std::fmt::Display for InvoiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "Not found: {s}"),
            Self::InvalidAmount(s) => write!(f, "Invalid amount: {s}"),
            Self::InvalidStatus(s) => write!(f, "Invalid invoice status: {s}"),
            Self::AlreadyPaid => write!(f, "Invoice is already paid"),
            Self::AlreadyVoided => write!(f, "Invoice is already voided"),
            Self::StripeError(e) => write!(f, "Stripe error: {e}"),
            Self::PdfGenerationError(e) => write!(f, "PDF generation error: {e}"),
        }
    }
}
impl std::error::Error for InvoiceError {}
