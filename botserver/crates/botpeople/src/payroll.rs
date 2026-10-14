use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::models::{Payroll, PayrollStatus};

type Storage = Arc<Mutex<HashMap<Uuid, Payroll>>>;

#[derive(Clone)]
pub struct PayrollService {
    storage: Storage,
}

impl PayrollService {
    pub fn new() -> Self {
        PayrollService {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn process_month(
        &self,
        employee_id: Uuid,
        gross_pay: f64,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Payroll, String> {
        let id = Uuid::new_v4();
        let deductions = self.calculate_deductions(gross_pay);
        let net_pay = gross_pay - deductions;
        let payroll = Payroll {
            id,
            employee_id,
            period_start,
            period_end,
            gross_pay,
            deductions,
            net_pay,
            status: PayrollStatus::Completed,
        };
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.insert(id, payroll.clone());
        Ok(payroll)
    }

    fn calculate_deductions(&self, gross_pay: f64) -> f64 {
        let inss = (gross_pay * 0.08).min(1000.0);
        let irrf = if gross_pay > 5000.0 {
            (gross_pay - 5000.0) * 0.15
        } else if gross_pay > 3000.0 {
            (gross_pay - 3000.0) * 0.075
        } else {
            0.0
        };
        (inss + irrf).round()
    }

    pub fn get_payslip(&self, id: Uuid) -> Result<Payroll, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        store.get(&id).cloned().ok_or_else(|| format!("Payroll not found: {id}"))
    }

    pub fn list_by_employee(&self, employee_id: Uuid) -> Result<Vec<Payroll>, String> {
        let store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let mut records: Vec<Payroll> = store
            .values()
            .filter(|p| p.employee_id == employee_id)
            .cloned()
            .collect();
        records.sort_by(|a, b| b.period_start.cmp(&a.period_start));
        Ok(records)
    }

    pub fn mark_paid(&self, id: Uuid) -> Result<Payroll, String> {
        let mut store = self.storage.lock().map_err(|e| format!("Lock error: {e}"))?;
        let payroll = store.get_mut(&id).ok_or_else(|| format!("Payroll not found: {id}"))?;
        payroll.status = PayrollStatus::Paid;
        Ok(payroll.clone())
    }

    pub fn generate_payslip_text(&self, id: Uuid) -> Result<String, String> {
        let payroll = self.get_payslip(id)?;
        let text = format!(
            "Payslip\nPeriod: {} - {}\nGross Pay: ${:.2}\nDeductions: ${:.2}\nNet Pay: ${:.2}\nStatus: {}",
            payroll.period_start,
            payroll.period_end,
            payroll.gross_pay,
            payroll.deductions,
            payroll.net_pay,
            payroll.status.as_str(),
        );
        Ok(text)
    }
}

impl Default for PayrollService {
    fn default() -> Self {
        Self::new()
    }
}
