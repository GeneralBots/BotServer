use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: Uuid,
    pub org_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: String,
    pub position: String,
    pub hire_date: NaiveDate,
    pub status: EmploymentStatus,
    pub salary: f64,
    pub manager_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmploymentStatus {
    Active,
    Inactive,
    Terminated,
    OnLeave,
}

impl EmploymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmploymentStatus::Active => "Active",
            EmploymentStatus::Inactive => "Inactive",
            EmploymentStatus::Terminated => "Terminated",
            EmploymentStatus::OnLeave => "OnLeave",
        }
    }

    pub fn from_label(s: &str) -> Option<EmploymentStatus> {
        match s {
            "Active" => Some(EmploymentStatus::Active),
            "Inactive" => Some(EmploymentStatus::Inactive),
            "Terminated" => Some(EmploymentStatus::Terminated),
            "OnLeave" => Some(EmploymentStatus::OnLeave),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrDepartment {
    pub id: Uuid,
    pub name: String,
    pub head_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendance {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub date: NaiveDate,
    pub clock_in: Option<DateTime<Utc>>,
    pub clock_out: Option<DateTime<Utc>>,
    pub hours_worked: f64,
    pub overtime: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub leave_type: LeaveType,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: LeaveStatus,
    pub approved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeaveType {
    Vacation,
    Sick,
    Personal,
    Maternity,
    Paternity,
    Bereavement,
    Other,
}

impl LeaveType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeaveType::Vacation => "Vacation",
            LeaveType::Sick => "Sick",
            LeaveType::Personal => "Personal",
            LeaveType::Maternity => "Maternity",
            LeaveType::Paternity => "Paternity",
            LeaveType::Bereavement => "Bereavement",
            LeaveType::Other => "Other",
        }
    }

    pub fn from_label(s: &str) -> Option<LeaveType> {
        match s {
            "Vacation" => Some(LeaveType::Vacation),
            "Sick" => Some(LeaveType::Sick),
            "Personal" => Some(LeaveType::Personal),
            "Maternity" => Some(LeaveType::Maternity),
            "Paternity" => Some(LeaveType::Paternity),
            "Bereavement" => Some(LeaveType::Bereavement),
            "Other" => Some(LeaveType::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeaveStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

impl LeaveStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            LeaveStatus::Pending => "Pending",
            LeaveStatus::Approved => "Approved",
            LeaveStatus::Rejected => "Rejected",
            LeaveStatus::Cancelled => "Cancelled",
        }
    }

    pub fn from_label(s: &str) -> Option<LeaveStatus> {
        match s {
            "Pending" => Some(LeaveStatus::Pending),
            "Approved" => Some(LeaveStatus::Approved),
            "Rejected" => Some(LeaveStatus::Rejected),
            "Cancelled" => Some(LeaveStatus::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payroll {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub gross_pay: f64,
    pub deductions: f64,
    pub net_pay: f64,
    pub status: PayrollStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayrollStatus {
    Processing,
    Completed,
    Paid,
    Failed,
}

impl PayrollStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PayrollStatus::Processing => "Processing",
            PayrollStatus::Completed => "Completed",
            PayrollStatus::Paid => "Paid",
            PayrollStatus::Failed => "Failed",
        }
    }

    pub fn from_label(s: &str) -> Option<PayrollStatus> {
        match s {
            "Processing" => Some(PayrollStatus::Processing),
            "Completed" => Some(PayrollStatus::Completed),
            "Paid" => Some(PayrollStatus::Paid),
            "Failed" => Some(PayrollStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReview {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub reviewer_id: Uuid,
    pub review_date: NaiveDate,
    pub rating: f64,
    pub comments: String,
    pub goals: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benefit {
    pub id: Uuid,
    pub employee_id: Uuid,
    pub benefit_type: String,
    pub value: f64,
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
}
